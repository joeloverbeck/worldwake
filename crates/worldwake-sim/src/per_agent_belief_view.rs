use crate::{
    ActionDefRegistry, ActionDuration, ActionInstance, ActionInstanceId, ActionPayload,
    DurationExpr, RecipeDefinition, RecipeRegistry, RuntimeBeliefView,
    estimate_duration_from_beliefs,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use worldwake_core::{
    AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState, BelievedInstitutionalClaim,
    CarryCapacity, CombatProfile, CommodityConsumableProfile, CommodityKind,
    CommodityValuationProfile, ContentionGrant, ControlSource, DemandObservation, DriveThresholds,
    EffectiveRight, EntityId, EntityKind, ExpectationStore, HomeostaticNeeds, InTransitOnEdge,
    InstitutionalBeliefKey, InstitutionalBeliefRead, IntentionDispositionProfile,
    JusticeDispositionProfile, LastSeenMemory, LoadUnits, MerchandiseProfile, MetabolismProfile,
    OfficeData, Permille, PlaceTag, PreferenceProfile, Quantity, RecipeId,
    RecipientKnowledgeStatus, RecordedViolation, ResourceSource, RouteExperience,
    SocialObservation, SourceReliability, StockStoragePolicy, TellMemoryKey, TellProfile,
    TellTopic, Tick, TickRange, ToldBeliefMemory, TradeDispositionProfile, UniqueItemKind,
    UtilityProfile, WorkstationTag, World, Wound, danger_ratio_permille, is_incapacitated,
    load_of_entity,
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
    recipe_registry: Option<&'w RecipeRegistry>,
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
        Self::new_at_tick_with_recipes(agent, current_tick, world, None, belief_store)
    }

    #[must_use]
    pub const fn new_with_recipes(
        agent: EntityId,
        world: &'w World,
        recipe_registry: &'w RecipeRegistry,
        belief_store: &'w AgentBeliefStore,
    ) -> Self {
        Self::new_at_tick_with_recipes(agent, Tick(0), world, Some(recipe_registry), belief_store)
    }

    #[must_use]
    pub const fn new_at_tick_with_recipes(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        recipe_registry: Option<&'w RecipeRegistry>,
        belief_store: &'w AgentBeliefStore,
    ) -> Self {
        Self {
            agent,
            current_tick,
            world,
            recipe_registry,
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
        Self::with_runtime_at_tick_with_recipes(
            agent,
            current_tick,
            world,
            None,
            belief_store,
            runtime,
        )
    }

    #[must_use]
    pub const fn with_runtime_with_recipes(
        agent: EntityId,
        world: &'w World,
        recipe_registry: &'w RecipeRegistry,
        belief_store: &'w AgentBeliefStore,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self::with_runtime_at_tick_with_recipes(
            agent,
            Tick(0),
            world,
            Some(recipe_registry),
            belief_store,
            runtime,
        )
    }

    #[must_use]
    pub const fn with_runtime_at_tick_with_recipes(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        recipe_registry: Option<&'w RecipeRegistry>,
        belief_store: &'w AgentBeliefStore,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self {
            agent,
            current_tick,
            world,
            recipe_registry,
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
        Self::from_world_at_tick_with_recipes(agent, current_tick, world, None)
    }

    #[must_use]
    pub fn from_world_with_recipes(
        agent: EntityId,
        world: &'w World,
        recipe_registry: &'w RecipeRegistry,
    ) -> Self {
        Self::from_world_at_tick_with_recipes(agent, Tick(0), world, Some(recipe_registry))
    }

    #[must_use]
    pub fn from_world_at_tick_with_recipes(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        recipe_registry: Option<&'w RecipeRegistry>,
    ) -> Self {
        let belief_store = world
            .get_component_agent_belief_store(agent)
            .expect("agents must have AgentBeliefStore before constructing PerAgentBeliefView");
        Self::new_at_tick_with_recipes(agent, current_tick, world, recipe_registry, belief_store)
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
        Self::with_runtime_from_world_at_tick_with_recipes(
            agent,
            current_tick,
            world,
            None,
            runtime,
        )
    }

    #[must_use]
    pub fn with_runtime_from_world_with_recipes(
        agent: EntityId,
        world: &'w World,
        recipe_registry: &'w RecipeRegistry,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self::with_runtime_from_world_at_tick_with_recipes(
            agent,
            Tick(0),
            world,
            Some(recipe_registry),
            runtime,
        )
    }

    #[must_use]
    pub fn with_runtime_from_world_at_tick_with_recipes(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        recipe_registry: Option<&'w RecipeRegistry>,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        let belief_store = world
            .get_component_agent_belief_store(agent)
            .expect("agents must have AgentBeliefStore before constructing PerAgentBeliefView");
        Self::with_runtime_at_tick_with_recipes(
            agent,
            current_tick,
            world,
            recipe_registry,
            belief_store,
            runtime,
        )
    }

    fn believed_entity(&self, entity: EntityId) -> Option<&BelievedEntityState> {
        (entity != self.agent)
            .then(|| self.belief_store.get_entity(&entity))
            .flatten()
    }

    fn knows_entity(&self, entity: EntityId) -> bool {
        entity == self.agent
            || self.believed_entity(entity).is_some()
            || self
                .belief_store
                .institutional_beliefs
                .values()
                .flat_map(|beliefs| beliefs.iter())
                .any(|belief| {
                    worldwake_core::institutional_claim_subject_entity(belief.claim) == entity
                })
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

    /// Find the alive controller of `facility` who is co-located at `place`.
    ///
    /// Uses authoritative state for the facility's location (facilities are
    /// physical infrastructure, always present at the place) and belief state
    /// for which agents the observer knows about at the place.
    fn facility_controller_at(&self, facility: EntityId, place: EntityId) -> Option<EntityId> {
        // Check if the facility is authoritatively at this place.
        if self.world.effective_place(facility) != Some(place) {
            return None;
        }
        // Find a believed-present agent who controls the facility.
        self.entities_at(place).into_iter().find(|entity| {
            self.entity_kind(*entity) == Some(EntityKind::Agent)
                && self.is_alive(*entity)
                && self.world.can_exercise_control(*entity, facility).is_ok()
        })
    }
}

fn adjusted_travel_ticks(
    base_ticks: NonZeroU32,
    edge_id: worldwake_core::TravelEdgeId,
    route_experience: Option<&RouteExperience>,
    preference_profile: Option<PreferenceProfile>,
) -> NonZeroU32 {
    let Some(profile) = preference_profile else {
        return base_ticks;
    };
    let Some(experience) = route_experience.and_then(|route| route.edges.get(&edge_id)) else {
        return base_ticks;
    };

    let danger_ratio = danger_ratio_permille(experience);
    if danger_ratio == 0 {
        return base_ticks;
    }

    let penalty_permille = u32::from(profile.route_caution_weight.value()) * danger_ratio / 1000;
    let effective_ticks = base_ticks.get() * (1000 + penalty_permille) / 1000;
    NonZeroU32::new(effective_ticks).unwrap()
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
            .or_else(|| {
                self.knows_entity(entity)
                    .then(|| self.world.effective_place(entity))
                    .flatten()
            })
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

    fn locally_observed_entities_at(&self, agent: EntityId, place: EntityId) -> Vec<EntityId> {
        if agent != self.agent || self.world.effective_place(agent) != Some(place) {
            return self.entities_at(place);
        }

        let mut entities = self.world.entities_effectively_at(place);
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

    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        (agent == self.agent).then_some(self.belief_store)
    }

    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        if agent != self.agent {
            return Vec::new();
        }

        self.belief_store.social_observations.clone()
    }

    fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
        if agent != self.agent {
            return Vec::new();
        }

        self.belief_store
            .institutional_beliefs
            .values()
            .flat_map(|beliefs| beliefs.iter().cloned())
            .collect()
    }

    fn factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        if entity == self.agent {
            return self.world.factions_of(entity);
        }

        self.known_institutional_beliefs(self.agent)
            .into_iter()
            .filter_map(|belief| match belief.claim {
                worldwake_core::InstitutionalClaim::FactionMembership {
                    faction,
                    member,
                    active: true,
                    ..
                } if member == entity => Some(faction),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        self.factions_of(entity)
            .into_iter()
            .filter(|faction| {
                self.world
                    .get_component_bandit_faction_policy(*faction)
                    .is_some()
            })
            .collect()
    }

    fn locally_observed_bandit_camp_faction_at(
        &self,
        agent: EntityId,
        place: EntityId,
    ) -> Option<EntityId> {
        if agent != self.agent || self.world.effective_place(agent) != Some(place) {
            return None;
        }

        self.world
            .get_component_bandit_camp(place)
            .map(|camp| camp.faction)
    }

    fn believed_activity_of(&self, entity: EntityId) -> Option<&worldwake_core::BelievedActivity> {
        self.believed_entity(entity)
            .and_then(|state| state.believed_activity.as_ref())
    }

    fn agents_active_at(
        &self,
        place: EntityId,
        domain: worldwake_core::ActionDomain,
        target: Option<EntityId>,
    ) -> Vec<EntityId> {
        let mut entities = self
            .belief_store
            .known_entities
            .iter()
            .filter_map(|(entity, state)| {
                (state.last_known_place == Some(place)
                    && state.believed_activity.as_ref().is_some_and(|activity| {
                        activity.action_domain == domain
                            && (target.is_none() || activity.target == target)
                    }))
                .then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
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

    fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
        self.recipe_registry
            .and_then(|registry| registry.get(recipe))
            .cloned()
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

    fn locally_observed_commodity_quantity(
        &self,
        agent: EntityId,
        holder: EntityId,
        kind: CommodityKind,
    ) -> Quantity {
        if agent != self.agent {
            return self.commodity_quantity(holder, kind);
        }

        let Some(agent_place) = self.world.effective_place(agent) else {
            return self.commodity_quantity(holder, kind);
        };
        if self.world.effective_place(holder) != Some(agent_place) {
            return self.commodity_quantity(holder, kind);
        }

        if let Some(source) = self.world.get_component_resource_source(holder)
            && source.commodity == kind
        {
            return source.available_quantity;
        }

        self.world
            .controlled_commodity_quantity_at_place(holder, agent_place, kind)
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

    fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        self.world
            .get_component_bandit_faction_policy(faction)
            .map(|policy| policy.flee_wound_threshold)
    }

    fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<std::num::NonZeroU32> {
        self.world
            .get_component_bandit_faction_policy(faction)
            .map(|policy| policy.establishment_duration_ticks)
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

    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        let accessible =
            self.knows_entity(entity) || self.world.owner_of(entity) == Some(self.agent);
        if !accessible {
            return Vec::new();
        }
        self.world.effective_rights(actor, entity)
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

    fn has_contention_policy(&self, entity: EntityId) -> bool {
        self.world.get_component_contention_policy(entity).is_some()
    }

    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        self.world
            .get_component_contention_queue(facility)
            .and_then(|queue| queue.position_of(actor))
    }

    fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
        self.world
            .get_component_contention_queue(facility)
            .and_then(|queue| queue.granted.as_ref())
    }

    fn contention_queue_is_full(&self, entity: EntityId) -> bool {
        let Some(policy) = self.world.get_component_contention_policy(entity) else {
            return false;
        };
        let Some(queue) = self.world.get_component_contention_queue(entity) else {
            return false;
        };
        policy
            .max_waiters
            .is_some_and(|limit| queue.waiting.len() >= usize::from(limit))
    }

    fn facility_queue_join_tick(&self, facility: EntityId, actor: EntityId) -> Option<Tick> {
        self.world
            .get_component_contention_queue(facility)
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
            .get_component_contention_disposition_profile(agent)
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

    fn stock_storage_policy(&self, facility: EntityId) -> Option<StockStoragePolicy> {
        self.knows_entity(facility)
            .then(|| {
                self.world
                    .get_component_stock_storage_policy(facility)
                    .cloned()
            })
            .flatten()
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

    fn locally_observed_is_dead(&self, agent: EntityId, entity: EntityId) -> bool {
        if agent != self.agent {
            return self.is_dead(entity);
        }

        let Some(agent_place) = self.world.effective_place(agent) else {
            return self.is_dead(entity);
        };
        if self.world.effective_place(entity) != Some(agent_place) {
            return self.is_dead(entity);
        }

        self.world.get_component_dead_at(entity).is_some()
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

    fn observation_fidelity(&self, agent: EntityId) -> Permille {
        self.world
            .get_component_perception_profile(agent)
            .map_or(Permille::new_unchecked(1000), |profile| {
                profile.observation_fidelity
            })
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

    fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_commodity_valuation_profile(agent)
                    .copied()
            })
            .flatten()
    }

    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        (agent == self.agent)
            .then(|| self.world.get_component_route_experience(agent).cloned())
            .flatten()
    }

    fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
        (agent == self.agent)
            .then(|| self.world.get_component_source_reliability(agent).cloned())
            .flatten()
    }

    fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_preference_profile(agent).copied())
            .flatten()
    }

    fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
        (agent == self.agent)
            .then(|| self.world.get_component_expectation_store(agent).cloned())
            .flatten()
    }

    fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_last_seen_memory(agent).cloned())
            .flatten()
    }

    fn utility_profile(&self, agent: EntityId) -> Option<UtilityProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_utility_profile(agent).cloned())
            .flatten()
    }

    fn patrol_profile(&self, agent: EntityId) -> Option<worldwake_core::PatrolProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_patrol_profile(agent).cloned())
            .flatten()
    }

    fn patrol_route(&self, agent: EntityId) -> Option<worldwake_core::PatrolRoute> {
        (agent == self.agent)
            .then(|| self.world.get_component_patrol_route(agent).cloned())
            .flatten()
    }

    fn pursuit_profile(&self, agent: EntityId) -> Option<worldwake_core::PursuitProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_pursuit_profile(agent).cloned())
            .flatten()
    }

    fn epistemic_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::EpistemicDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_epistemic_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn theft_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::TheftDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_theft_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_justice_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn intention_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<IntentionDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_intention_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
        self.world.topology().shortest_path(from, to).is_some()
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
        topic: &TellTopic,
    ) -> Option<ToldBeliefMemory> {
        if actor != self.agent {
            return None;
        }

        let profile = self.tell_profile(actor)?;
        self.belief_store
            .told_belief_memory(
                &TellMemoryKey {
                    counterparty,
                    topic: *topic,
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
        topic: &TellTopic,
    ) -> Option<RecipientKnowledgeStatus> {
        if actor != self.agent {
            return None;
        }

        let profile = self.tell_profile(actor)?;
        let current_state = self
            .belief_store
            .shared_tell_state_for_topic(topic, profile.max_relay_chain_len)?;
        Some(self.belief_store.recipient_knowledge_status(
            &TellMemoryKey {
                counterparty,
                topic: *topic,
            },
            &current_state,
            self.current_tick,
            &profile,
        ))
    }

    fn ask_witness_memory(
        &self,
        actor: EntityId,
        key: &worldwake_core::AskWitnessMemoryKey,
    ) -> Option<worldwake_core::AskWitnessMemory> {
        if actor != self.agent {
            return None;
        }

        let profile = self.epistemic_disposition_profile(actor)?;
        self.belief_store
            .ask_witness_memory(key, self.current_tick, profile.ask_memory_retention_ticks)
            .cloned()
    }

    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_combat_profile(agent).copied())
            .flatten()
    }

    fn violation_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::ViolationDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_violation_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
        if agent != self.agent {
            return Vec::new();
        }

        self.world
            .get_component_violation_memory(agent)
            .map(|memory| {
                memory
                    .unresolved_records(self.current_tick)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
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

    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::ItemLot))
            .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
            .filter(|entity| self.world.has_component_sale_listing(*entity))
            .filter(|entity| {
                // Facility-based visibility: lot must be displayed
                // (StockAssignment::Displayed) and the facility controller
                // must be alive and co-located at this place.
                self.world
                    .get_component_stock_assignment(*entity)
                    .is_some_and(|assignment| {
                        assignment.kind == worldwake_core::StockAssignmentKind::Displayed
                            && self
                                .facility_controller_at(assignment.facility, place)
                                .is_some()
                    })
            })
            .collect()
    }

    fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
        if !self.world.has_component_sale_listing(lot) {
            return None;
        }
        // Derive seller from facility control rather than direct possession.
        let assignment = self.world.get_component_stock_assignment(lot)?;
        if assignment.kind != worldwake_core::StockAssignmentKind::Displayed {
            return None;
        }
        let place = self.effective_place(lot)?;
        self.facility_controller_at(assignment.facility, place)
    }

    fn has_sale_listing(&self, lot: EntityId) -> bool {
        self.world.has_component_sale_listing(lot)
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

    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.belief_store.believed_office_holder(office)
    }

    fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        self.belief_store.believed_force_controller(office)
    }

    fn believed_membership(
        &self,
        faction: EntityId,
        member: EntityId,
    ) -> InstitutionalBeliefRead<bool> {
        self.belief_store.believed_membership(faction, member)
    }

    fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.belief_store.believed_faction_rally_point(faction)
    }

    fn offices_contested_by(&self, claimant: EntityId) -> Vec<EntityId> {
        if claimant != self.agent {
            return Vec::new();
        }

        self.world.offices_contested_by(claimant)
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

    fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.belief_store
            .believed_support_declaration(office, supporter)
    }

    fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        self.belief_store
            .believed_support_declarations_for_office(office)
    }

    fn institutional_belief_claims(
        &self,
        agent: EntityId,
        key: InstitutionalBeliefKey,
    ) -> Vec<BelievedInstitutionalClaim> {
        if agent != self.agent {
            return Vec::new();
        }
        self.belief_store
            .institutional_beliefs
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        if entity == self.agent {
            return self.world.get_component_in_transit_on_edge(entity).cloned();
        }

        None
    }

    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        let route_experience = self.route_experience(self.agent);
        let preference_profile = self.preference_profile(self.agent);

        self.world
            .topology()
            .outgoing_edges(place)
            .iter()
            .filter_map(|edge_id| self.world.topology().edge(*edge_id))
            .map(|edge| {
                let base_ticks = NonZeroU32::new(edge.travel_time_ticks()).unwrap();
                (
                    edge.to(),
                    adjusted_travel_ticks(
                        base_ticks,
                        edge.id(),
                        route_experience.as_ref(),
                        preference_profile,
                    ),
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
        ActionDef, ActionDefRegistry, ActionDuration, ActionHandlerId, ActionInstance,
        ActionInstanceId, ActionPayload, ActionStatus, Constraint, DurationExpr, GoalBeliefView,
        Interruptibility, Precondition, ReservationReq, RuntimeBeliefView, TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState,
        BodyCostPerTick, BodyPart, CauseRef, CombatProfile, CommodityKind, ControlSource,
        EdgeExperience, EffectiveRight, EntityId, EntityKind, EventLog, ExpectationBasis,
        ExpectationId, ExpectationRecord, ExpectationState, ExpectationStore, FactionData,
        FactionPurpose, InstitutionalBeliefKey, InstitutionalBeliefRead, InstitutionalClaim,
        InstitutionalKnowledgeSource, LastSeenMemory, LastSeenProvenance, LastSeenRecord,
        OfficeData, PerceptionProfile, Permille, Place, PlaceTag, PreferenceProfile, Quantity,
        RecipientKnowledgeStatus, RecordData, RecordKind, ResourceSource, RightKind,
        RouteExperience, SuccessionLaw, TellMemoryKey, TellTopic, Tick, ToldBeliefMemory, Topology,
        TravelEdge, TravelEdgeId, UtilityProfile, VisibilitySpec, WitnessData, WorkstationMarker,
        WorkstationTag, World, WorldTxn, Wound, WoundCause, WoundId, build_believed_entity_state,
        build_prototype_world,
        test_utils::{
            sample_commodity_valuation_profile, sample_preference_profile, sample_route_experience,
            sample_source_reliability,
        },
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
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(observed_tick),
            source: worldwake_core::PerceptionSource::DirectObservation,
        }
    }

    fn entity_belief_with_activity(
        place: worldwake_core::EntityId,
        domain: ActionDomain,
        target: Option<worldwake_core::EntityId>,
        observed_tick: u64,
    ) -> BelievedEntityState {
        let mut state = entity_belief(place, true, 0, observed_tick);
        state.believed_activity = Some(worldwake_core::BelievedActivity {
            action_domain: domain,
            target,
            observed_tick: Tick(observed_tick),
        });
        state
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
            attention_cost: worldwake_core::Permille::ZERO,
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
    fn self_expectation_and_last_seen_queries_are_authoritative_only_for_self() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 5);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            let mut expectation_store = ExpectationStore::default();
            expectation_store.records.insert(
                ExpectationId(7),
                ExpectationRecord {
                    id: ExpectationId(7),
                    owner: agent,
                    subject: other,
                    expected_place: place,
                    deadline_tick: Tick(12),
                    grace_ticks: 3,
                    basis: ExpectationBasis::SocialPromise,
                    state: ExpectationState::Active,
                    created_tick: Tick(5),
                },
            );
            txn.set_component_expectation_store(agent, expectation_store)
                .unwrap();
            txn.set_component_last_seen_memory(
                agent,
                LastSeenMemory {
                    records: BTreeMap::from([(
                        other,
                        LastSeenRecord {
                            subject: other,
                            place,
                            observed_tick: Tick(4),
                            source: agent,
                            provenance: LastSeenProvenance::DirectObservation,
                        },
                    )]),
                    capacity: 9,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let view = PerAgentBeliefView::from_world(agent, &world);
        let expectation_store = GoalBeliefView::expectation_store(&view, agent)
            .expect("self expectation store should be visible");
        let last_seen_memory = GoalBeliefView::last_seen_memory(&view, agent)
            .expect("self last-seen memory should be visible");

        assert_eq!(expectation_store.records.len(), 1);
        assert_eq!(
            expectation_store.records.get(&ExpectationId(7)),
            Some(&ExpectationRecord {
                id: ExpectationId(7),
                owner: agent,
                subject: other,
                expected_place: place,
                deadline_tick: Tick(12),
                grace_ticks: 3,
                basis: ExpectationBasis::SocialPromise,
                state: ExpectationState::Active,
                created_tick: Tick(5),
            })
        );
        assert_eq!(last_seen_memory.capacity, 9);
        assert_eq!(
            last_seen_memory.records.get(&other),
            Some(&LastSeenRecord {
                subject: other,
                place,
                observed_tick: Tick(4),
                source: agent,
                provenance: LastSeenProvenance::DirectObservation,
            })
        );
        assert_eq!(GoalBeliefView::expectation_store(&view, other), None);
        assert_eq!(GoalBeliefView::last_seen_memory(&view, other), None);
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
        use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, believed_merchant, _hidden_merchant, listed_lot, hidden_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let believed_merchant = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            let hidden_merchant = txn.create_agent("Hidden", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(believed_merchant, place).unwrap();
            txn.set_ground_location(hidden_merchant, place).unwrap();

            // Believed merchant: facility with display container, lot staged
            let (facility1, _stock1, display1) = txn
                .create_merchant_facility(
                    place,
                    believed_merchant,
                    LoadUnits(200),
                    Some(LoadUnits(100)),
                )
                .unwrap();
            let display1 = display1.unwrap();
            let listed_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(5))
                .unwrap();
            txn.put_into_container(listed_lot, display1).unwrap();
            txn.set_component_stock_assignment(
                listed_lot,
                StockAssignment {
                    facility: facility1,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(
                listed_lot,
                worldwake_core::SaleListing {
                    listed_at: worldwake_core::Tick(0),
                },
            )
            .unwrap();

            // Hidden merchant: same setup but agent won't have beliefs about it
            let (facility2, _stock2, display2) = txn
                .create_merchant_facility(
                    place,
                    hidden_merchant,
                    LoadUnits(200),
                    Some(LoadUnits(100)),
                )
                .unwrap();
            let display2 = display2.unwrap();
            let hidden_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(5))
                .unwrap();
            txn.put_into_container(hidden_lot, display2).unwrap();
            txn.set_component_stock_assignment(
                hidden_lot,
                StockAssignment {
                    facility: facility2,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(
                hidden_lot,
                worldwake_core::SaleListing {
                    listed_at: worldwake_core::Tick(0),
                },
            )
            .unwrap();

            commit_txn(txn);
            (
                agent,
                believed_merchant,
                hidden_merchant,
                listed_lot,
                hidden_lot,
            )
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(believed_merchant, entity_belief(place, true, 3, 5));
        beliefs.update_entity(listed_lot, entity_belief(place, true, 3, 5));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        // Only the listed lot of the believed merchant is visible; hidden merchant's lot is not.
        // The believed merchant is alive and co-located, so their facility's displayed lot
        // passes the facility_controller_at check.
        assert_eq!(
            RuntimeBeliefView::listed_sale_lots_at(&view, place, CommodityKind::Bread),
            vec![listed_lot]
        );
        assert_eq!(
            RuntimeBeliefView::seller_for_sale_lot(&view, listed_lot),
            Some(believed_merchant)
        );
        // Hidden lot's seller is not discoverable (agent doesn't know about hidden_lot)
        assert_eq!(
            RuntimeBeliefView::seller_for_sale_lot(&view, hidden_lot),
            None
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
    fn believed_activity_of_reads_subjective_activity_beliefs_only() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let unknown = entity(999);
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let observed = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(observed, place).unwrap();
            commit_txn(txn);
            (agent, observed)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            other,
            entity_belief_with_activity(place, ActionDomain::Production, Some(entity(40)), 8),
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::believed_activity_of(&view, other),
            Some(&worldwake_core::BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(entity(40)),
                observed_tick: Tick(8),
            })
        );
        assert_eq!(RuntimeBeliefView::believed_activity_of(&view, agent), None);
        assert_eq!(
            RuntimeBeliefView::believed_activity_of(&view, unknown),
            None
        );
    }

    #[test]
    fn agents_active_at_filters_believed_entities_by_place_domain_and_target() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = places[1];
        let source = entity(40);
        let other_source = entity(41);
        let (agent, a, b, c, d) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let a = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let b = txn.create_agent("Cora", ControlSource::Ai).unwrap();
            let c = txn.create_agent("Dain", ControlSource::Ai).unwrap();
            let d = txn.create_agent("Edda", ControlSource::Ai).unwrap();
            for entity in [agent, a, b, c, d] {
                txn.set_ground_location(entity, place).unwrap();
            }
            commit_txn(txn);
            (agent, a, b, c, d)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            a,
            entity_belief_with_activity(place, ActionDomain::Production, Some(source), 8),
        );
        beliefs.update_entity(
            b,
            entity_belief_with_activity(place, ActionDomain::Trade, Some(source), 8),
        );
        beliefs.update_entity(
            c,
            entity_belief_with_activity(place, ActionDomain::Production, Some(other_source), 8),
        );
        beliefs.update_entity(
            d,
            entity_belief_with_activity(other_place, ActionDomain::Production, Some(source), 8),
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::agents_active_at(&view, place, ActionDomain::Production, None),
            vec![a, c]
        );
        assert_eq!(
            RuntimeBeliefView::agents_active_at(
                &view,
                place,
                ActionDomain::Production,
                Some(source)
            ),
            vec![a]
        );
        assert_eq!(
            RuntimeBeliefView::agents_active_at(&view, place, ActionDomain::Trade, Some(source)),
            vec![b]
        );
        assert!(
            RuntimeBeliefView::agents_active_at(
                &view,
                other_place,
                ActionDomain::Trade,
                Some(source)
            )
            .is_empty()
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
                topic: TellTopic::EntityBelief { subject },
            },
            ToldBeliefMemory {
                shared_state: worldwake_core::SharedTellState::EntityBelief(
                    worldwake_core::to_shared_belief_snapshot(&stale_belief),
                ),
                told_tick: Tick(4),
            },
        );
        let topic = TellTopic::EntityBelief { subject };

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(6), &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&view, agent, listener, &topic)
                .map(|m| m.told_tick),
            Some(Tick(4))
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&view, agent, listener, &topic),
            Some(RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief)
        );

        let expired_view = PerAgentBeliefView::new_at_tick(agent, Tick(60), &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&expired_view, agent, listener, &topic),
            None
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&expired_view, agent, listener, &topic,),
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
                topic: TellTopic::EntityBelief { subject },
            },
            ToldBeliefMemory {
                shared_state: worldwake_core::SharedTellState::EntityBelief(
                    worldwake_core::to_shared_belief_snapshot(&entity_belief(place, true, 1, 4)),
                ),
                told_tick: Tick(4),
            },
        );
        let topic = TellTopic::EntityBelief { subject };

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
            RuntimeBeliefView::told_belief_memory(&view, other, listener, &topic),
            None
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&view, other, listener, &topic),
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
    fn commodity_valuation_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = sample_commodity_valuation_profile();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_commodity_valuation_profile(agent, profile)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::commodity_valuation_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::commodity_valuation_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn commodity_valuation_profile_returns_none_when_component_missing() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            agent
        };
        {
            let mut txn = new_txn(&mut world, 2);
            txn.clear_component_preference_profile(agent).unwrap();
            commit_txn(txn);
        }

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::commodity_valuation_profile(&view, agent),
            None
        );
        assert_eq!(
            GoalBeliefView::commodity_valuation_profile(&view, agent),
            None
        );
    }

    #[test]
    fn route_experience_returns_actor_experience_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let route_experience = sample_route_experience();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_route_experience(agent, route_experience.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::route_experience(&view, agent),
            Some(route_experience.clone())
        );
        assert_eq!(
            GoalBeliefView::route_experience(&view, agent),
            Some(route_experience)
        );
    }

    #[test]
    fn route_experience_returns_none_when_component_missing() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::route_experience(&view, agent), None);
        assert_eq!(GoalBeliefView::route_experience(&view, agent), None);
    }

    #[test]
    fn source_reliability_returns_actor_reliability_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let source_reliability = sample_source_reliability();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_source_reliability(agent, source_reliability.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::source_reliability(&view, agent),
            Some(source_reliability.clone())
        );
        assert_eq!(
            GoalBeliefView::source_reliability(&view, agent),
            Some(source_reliability)
        );
    }

    #[test]
    fn source_reliability_returns_none_when_component_missing() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::source_reliability(&view, agent), None);
        assert_eq!(GoalBeliefView::source_reliability(&view, agent), None);
    }

    #[test]
    fn preference_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = sample_preference_profile();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_preference_profile(agent, profile)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::preference_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::preference_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn preference_profile_returns_default_for_live_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::preference_profile(&view, agent),
            Some(PreferenceProfile::default())
        );
        assert_eq!(
            GoalBeliefView::preference_profile(&view, agent),
            Some(PreferenceProfile::default())
        );
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

    fn place(name: &str, tag: PlaceTag) -> Place {
        Place {
            name: name.to_string(),
            capacity: None,
            tags: [tag].into_iter().collect(),
        }
    }

    fn travel_cost_test_world() -> (World, EntityId, EntityId, TravelEdgeId, NonZeroU32) {
        let origin = entity(1);
        let destination = entity(2);
        let edge_id = TravelEdgeId(10);
        let base_ticks = NonZeroU32::new(10).unwrap();
        let mut topology = Topology::new();
        topology
            .add_place(origin, place("Origin", PlaceTag::Village))
            .unwrap();
        topology
            .add_place(destination, place("Destination", PlaceTag::Farm))
            .unwrap();
        topology
            .add_edge(
                TravelEdge::new(edge_id, origin, destination, base_ticks.get(), None).unwrap(),
            )
            .unwrap();

        (
            World::new(topology).unwrap(),
            origin,
            destination,
            edge_id,
            base_ticks,
        )
    }

    #[test]
    fn adjacent_places_with_travel_ticks_returns_raw_cost_without_route_experience() {
        let (mut world, origin, destination, _edge_id, base_ticks) = travel_cost_test_world();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_preference_profile(agent, sample_preference_profile())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, origin),
            vec![(destination, base_ticks)]
        );
    }

    #[test]
    fn adjacent_places_with_travel_ticks_uses_default_preference_profile_for_live_agent() {
        let (mut world, origin, destination, edge_id, base_ticks) = travel_cost_test_world();
        let route_experience = RouteExperience {
            edges: BTreeMap::from([(
                edge_id,
                EdgeExperience {
                    safe_trips: 1,
                    hostile_encounters: 1,
                    last_travel_tick: Tick(9),
                },
            )]),
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_route_experience(agent, route_experience)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let expected_ticks = NonZeroU32::new(
            base_ticks.get()
                * (1000
                    + u32::from(PreferenceProfile::default().route_caution_weight.value()) * 500
                        / 1000)
                / 1000,
        )
        .unwrap();

        assert_eq!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, origin),
            vec![(destination, expected_ticks)]
        );
    }

    #[test]
    fn adjacent_places_with_travel_ticks_leaves_safe_routes_unpenalized() {
        let (mut world, origin, destination, edge_id, base_ticks) = travel_cost_test_world();
        let route_experience = RouteExperience {
            edges: BTreeMap::from([(
                edge_id,
                EdgeExperience {
                    safe_trips: 5,
                    hostile_encounters: 0,
                    last_travel_tick: Tick(9),
                },
            )]),
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_route_experience(agent, route_experience)
                .unwrap();
            txn.set_component_preference_profile(agent, sample_preference_profile())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, origin),
            vec![(destination, base_ticks)]
        );
    }

    #[test]
    fn adjacent_places_with_travel_ticks_applies_hostile_route_penalty() {
        let (mut world, origin, destination, edge_id, base_ticks) = travel_cost_test_world();
        let route_experience = RouteExperience {
            edges: BTreeMap::from([(
                edge_id,
                EdgeExperience {
                    safe_trips: 1,
                    hostile_encounters: 1,
                    last_travel_tick: Tick(9),
                },
            )]),
        };
        let profile = sample_preference_profile();
        let expected_ticks = NonZeroU32::new(
            base_ticks.get()
                * (1000 + u32::from(profile.route_caution_weight.value()) * 500 / 1000)
                / 1000,
        )
        .unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_route_experience(agent, route_experience)
                .unwrap();
            txn.set_component_preference_profile(agent, profile)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, origin),
            vec![(destination, expected_ticks)]
        );
    }

    #[test]
    fn adjacent_places_with_travel_ticks_applies_maximum_hostile_penalty() {
        let (mut world, origin, destination, edge_id, base_ticks) = travel_cost_test_world();
        let route_experience = RouteExperience {
            edges: BTreeMap::from([(
                edge_id,
                EdgeExperience {
                    safe_trips: 0,
                    hostile_encounters: 3,
                    last_travel_tick: Tick(9),
                },
            )]),
        };
        let profile = PreferenceProfile {
            route_caution_weight: Permille::new(800).unwrap(),
            ..sample_preference_profile()
        };
        let expected_ticks = NonZeroU32::new(
            base_ticks.get()
                * (1000 + u32::from(profile.route_caution_weight.value()) * 1000 / 1000)
                / 1000,
        )
        .unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_route_experience(agent, route_experience)
                .unwrap();
            txn.set_component_preference_profile(agent, profile)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, origin),
            vec![(destination, expected_ticks)]
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
                body_cost_override: None,
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
    #[allow(clippy::too_many_lines)]
    fn political_queries_use_belief_backed_institutional_reads_and_actor_private_relations() {
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
                    seat: place,
                    jurisdiction: BTreeSet::from([place]),
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
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionMembersOf { faction },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::FactionMembership {
                    faction,
                    member: agent,
                    active: true,
                    effective_tick: Tick(3),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor {
                supporter: agent,
                office,
            },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::SupportDeclaration {
                    office,
                    supporter: agent,
                    candidate: Some(holder),
                    effective_tick: Tick(3),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::office_data(&view, office)
                .unwrap()
                .jurisdiction,
            BTreeSet::from([place])
        );
        assert_eq!(
            RuntimeBeliefView::office_data(&view, office).unwrap().seat,
            place
        );
        assert_eq!(
            RuntimeBeliefView::believed_office_holder(&view, office),
            InstitutionalBeliefRead::Certain(Some(holder))
        );
        assert_eq!(
            RuntimeBeliefView::believed_membership(&view, faction, agent),
            InstitutionalBeliefRead::Certain(true)
        );
        assert_eq!(
            RuntimeBeliefView::loyalty_to(&view, agent, holder),
            Some(Permille::new(620).unwrap())
        );
        assert_eq!(
            RuntimeBeliefView::believed_support_declaration(&view, office, agent),
            InstitutionalBeliefRead::Certain(Some(holder))
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
    fn believed_force_controller_reads_from_institutional_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (agent, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Believer", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (agent, office)
        };
        let mut beliefs = AgentBeliefStore::new();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::ForceControllerOf { office },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::ForceControl {
                    office,
                    controller: Some(entity(173)),
                    contested: false,
                    effective_tick: Tick(6),
                },
                source: worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(6),
                learned_at: Some(entity(174)),
            }],
        );
        let mut txn = new_txn(&mut world, 1);
        txn.set_component_agent_belief_store(agent, beliefs)
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);

        let view = PerAgentBeliefView::new_at_tick(
            agent,
            Tick(6),
            &world,
            world.get_component_agent_belief_store(agent).unwrap(),
        );

        assert_eq!(
            RuntimeBeliefView::believed_force_controller(&view, office),
            InstitutionalBeliefRead::Certain((Some(entity(173)), false))
        );
        assert_eq!(
            GoalBeliefView::believed_force_controller(&view, office),
            InstitutionalBeliefRead::Certain((Some(entity(173)), false))
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
            vec![(supporter, InstitutionalBeliefRead::Certain(Some(candidate)),)]
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
    fn locally_observed_commodity_quantity_excludes_remote_possessions() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let local_place = places[0];
        let remote_place = places[1];
        let (observer, holder, victim, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            let victim = txn.create_agent("Victim", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, local_place).unwrap();
            txn.set_ground_location(holder, local_place).unwrap();
            txn.set_ground_location(victim, remote_place).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(lot, local_place).unwrap();
            txn.set_owner(lot, victim).unwrap();
            txn.set_possessor(lot, holder).unwrap();
            commit_txn(txn);
            (observer, holder, victim, lot)
        };

        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(observer, remote_place).unwrap();
            txn.set_ground_location(holder, remote_place).unwrap();
            commit_txn(txn);
        }

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(holder, entity_belief(remote_place, true, 0, 10));
        beliefs.update_entity(lot, entity_belief(local_place, true, 2, 10));
        beliefs.update_entity(victim, entity_belief(remote_place, true, 0, 10));

        let view = PerAgentBeliefView::new(observer, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::locally_observed_commodity_quantity(
                &view,
                observer,
                holder,
                CommodityKind::Bread,
            ),
            Quantity(0),
            "co-located observation should exclude commodity the holder cannot access from the current place"
        );
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
    fn believed_rights_returns_rights_for_known_entity() {
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
        beliefs.update_entity(lot, entity_belief(place, true, 3, 2));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::believed_rights(&view, agent, lot),
            vec![EffectiveRight {
                kind: RightKind::Ownership,
                via: None,
            }]
        );
    }

    #[test]
    fn believed_rights_returns_empty_for_unknown_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(1), place, Some(other))
                .unwrap();
            commit_txn(txn);
            (agent, other, lot)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(RuntimeBeliefView::believed_rights(&view, other, lot).is_empty());
    }

    #[test]
    fn believed_rights_surfaces_jurisdiction_without_control() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let seat = places[0];
        let jurisdiction_place = *places.get(1).unwrap_or(&seat);
        let (observer, holder, office, item) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Marshal", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, seat).unwrap();
            txn.set_ground_location(holder, seat).unwrap();
            let office = txn.create_office("Marshal Seat").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Marshal".to_string(),
                    seat,
                    jurisdiction: BTreeSet::from([seat, jurisdiction_place]),
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 8,
                    vacancy_since: None,
                },
            )
            .unwrap();
            create_record(&mut txn, seat, observer, RecordKind::OfficeRegister);
            txn.assign_office(office, holder).unwrap();
            let item = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            txn.set_ground_location(item, jurisdiction_place).unwrap();
            commit_txn(txn);
            (observer, holder, office, item)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(item, entity_belief(jurisdiction_place, true, 0, 2));

        let view = PerAgentBeliefView::new(observer, &world, &beliefs);
        assert!(!RuntimeBeliefView::can_control(&view, holder, item));
        assert_eq!(
            RuntimeBeliefView::believed_rights(&view, holder, item),
            vec![EffectiveRight {
                kind: RightKind::JurisdictionalAuthority,
                via: Some(office),
            }]
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
    fn believed_membership_reads_from_institutional_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, faction) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let faction = txn.create_faction("River Pact").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            (agent, faction)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionMembersOf { faction },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::FactionMembership {
                    faction,
                    member: agent,
                    active: true,
                    effective_tick: Tick(3),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::believed_membership(&view, faction, agent),
            InstitutionalBeliefRead::Certain(true)
        );
    }

    #[test]
    fn believed_faction_rally_point_reads_from_institutional_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let rally_place = *places.get(1).unwrap_or(&place);
        let (agent, faction, rally_place) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let faction = txn.create_faction("River Pact").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            (agent, faction, rally_place)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::FactionRallyPointOf { faction },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::FactionRallyPoint {
                    faction,
                    rally_place: Some(rally_place),
                    effective_tick: Tick(3),
                },
                source: InstitutionalKnowledgeSource::DirectObservation,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::believed_faction_rally_point(&view, faction),
            InstitutionalBeliefRead::Certain(Some(rally_place))
        );
    }

    #[test]
    fn institutional_belief_claims_returns_claims_from_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let office = txn.create_office("Town Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            (agent, office)
        };

        let holder = {
            let mut txn = new_txn(&mut world, 2);
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            commit_txn(txn);
            holder
        };

        let mut beliefs = AgentBeliefStore::new();
        let key = InstitutionalBeliefKey::OfficeHolderOf { office };
        let claim = worldwake_core::BelievedInstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office,
                holder: Some(holder),
                effective_tick: Tick(1),
            },
            source: InstitutionalKnowledgeSource::WitnessedEvent,
            learned_tick: Tick(2),
            learned_at: Some(place),
        };
        beliefs
            .institutional_beliefs
            .insert(key, vec![claim.clone()]);

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let result = GoalBeliefView::institutional_belief_claims(&view, agent, key);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], claim);
    }

    #[test]
    fn institutional_belief_claims_returns_empty_for_other_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let office = txn.create_office("Town Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        // Querying for a different agent should return empty.
        let key = InstitutionalBeliefKey::OfficeHolderOf { office };
        let result = GoalBeliefView::institutional_belief_claims(&view, other, key);
        assert!(result.is_empty());
    }
}
