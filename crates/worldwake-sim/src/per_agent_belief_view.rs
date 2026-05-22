use crate::{
    ActionDefRegistry, ActionDuration, ActionInstance, ActionInstanceId, ActionPayload, BeliefRead,
    BelievedAuthorityView, CombatBeliefView, ControlBeliefView, DurationExpr, EconomicBeliefView,
    EntityBeliefView, FacilityBeliefView, InventoryBeliefView, LocalPhysicalObservationView,
    ObservationSource, ObservedRead, PoliticalBeliefView, ProfileBeliefView, RecipeDefinition,
    RecipeRegistry, RuntimeBeliefView, SocialBeliefView, SpatialBeliefView, TemporalBeliefView,
    estimate_duration_from_beliefs,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use worldwake_core::{
    AgentBeliefStore, AgentSchemaContextProfile, ArtifactPostingProfile, BeliefConfidencePolicy,
    BelievedEntityState, BelievedInstitutionalClaim, CarryCapacity, CognitiveProfile,
    CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityValuationProfile,
    ContentionGrant, ControlSource, DemandObservation, DeprivationExposure, DisposalProfile,
    DiversificationProfile, DriveEscalationProfile, DriveThresholds, EffectiveRight,
    EntityBeliefAspect, EntityId, EntityKind, ExpectationStore, HomeostaticNeedId,
    HomeostaticNeeds, InTransitOnEdge, InstitutionalBeliefKey, InstitutionalBeliefRead,
    IntentionDispositionProfile, JusticeDispositionProfile, LastHarvestTrace,
    LastProactiveExplorationTick, LastSeenMemory, LastSeenProvenance, LatrineFullness,
    LawAbidingProfile, LoadUnits, MerchandiseProfile, MetabolismProfile,
    ObligationExecutionTracker, ObligationSatiationProfile, OfficeData, PerceptionProfile,
    PerceptionSource, Permille, PlaceDirtiness, PlaceTag, PortfolioWeightsProfile,
    PreferenceProfile, Quantity, RecipeId, RecipientKnowledgeStatus, RecordedViolation,
    ResourceExtractionQueues, ResourceSource, RewardEncumbrance, RiskWeightProfile,
    RouteExperience, RoutePreferenceProfile, SleepQualityProfile, SocialObservation,
    SourceReliability, StockStoragePolicy, SubstitutePreferences, SurveyMemory, TellMemoryKey,
    TellProfile, TellTopic, TestimonyTrustProfile, Tick, TickRange, ToldBeliefMemory,
    TradeDispositionProfile, UniqueItemKind, UtilityProfile, WashBasinState, WorkstationTag, World,
    Wound, danger_ratio_permille, is_incapacitated, load_of_entity,
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

    fn believed_contention_state_of(
        &self,
        entity: EntityId,
    ) -> Option<worldwake_core::BelievedContentionState> {
        self.believed_entity(entity)
            .and_then(|state| state.believed_contention)
    }

    /// True when the observing agent is physically co-located with `entity`.
    ///
    /// Used to gate observability of directly perceivable physical properties
    /// of co-located entities (kind, item-lot commodity/quantity, workstation
    /// tag, resource source, container contents). These are properties a
    /// correct perception pipeline would necessarily deliver on the same tick.
    ///
    /// This helper MUST NOT be used to gate social/relational knowledge
    /// (ownership, rights, institutional claims). Those require an explicit
    /// belief entry under FND-14/FND-15 and are gated separately in
    /// `believed_owner_of` and `believed_rights`.
    fn has_authoritative_local_visibility(&self, entity: EntityId) -> bool {
        let Some(agent_place) = self.world.effective_place(self.agent) else {
            return false;
        };
        self.world.effective_place(entity) == Some(agent_place)
    }

    fn knows_entity(&self, entity: EntityId) -> bool {
        entity == self.agent
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent)
            || self.believed_entity(entity).is_some()
            || self
                .belief_store
                .institutional_beliefs
                .values()
                .flat_map(|beliefs| beliefs.iter())
                .any(|belief| {
                    worldwake_core::institutional_claim_subject_entity(belief.claim) == entity
                })
            || self
                .world
                .get_component_last_seen_memory(self.agent)
                .is_some_and(|memory| memory.records.contains_key(&entity))
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

    fn believed_entity_authority_claim(
        &self,
        subject: EntityId,
        aspect: EntityBeliefAspect,
    ) -> BeliefRead<EntityId> {
        let threshold = self.claim_confidence_threshold(self.agent);
        let policy = self.belief_confidence_policy(self.agent);
        let Some(best) = crate::belief_view::project_claims_into_belief_set(
            self.belief_store
                .get_entity_claims(&subject)
                .into_iter()
                .flatten()
                .filter_map(|claim| {
                    crate::belief_view::entity_claim_value(claim, aspect)
                        .and_then(|entity| entity.map(|entity| (claim.clone(), entity)))
                }),
            self.current_tick,
            threshold,
            &policy,
        )
        .best
        else {
            return BeliefRead::Unknown;
        };

        match best.status {
            crate::belief_view::BeliefStatus::Stale => BeliefRead::Stale(best),
            crate::belief_view::BeliefStatus::Contradicted
            | crate::belief_view::BeliefStatus::Disputed => BeliefRead::Unknown,
            crate::belief_view::BeliefStatus::Certain
            | crate::belief_view::BeliefStatus::Probable => BeliefRead::Known(best),
        }
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

impl ControlBeliefView for PerAgentBeliefView<'_> {
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        // Effective rights are a social/jurisdictional fact (FND-14/FND-15).
        // Require an explicit belief gate instead of co-location or live owner
        // / possessor relations.
        let accessible = entity == self.agent || self.believed_entity(entity).is_some();
        if !accessible {
            return Vec::new();
        }
        self.world.effective_rights(actor, entity)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        let accessible = entity == self.agent || self.believed_entity(entity).is_some();
        if !accessible {
            return false;
        }
        self.world.can_exercise_control(actor, entity).is_ok()
    }

    fn has_control(&self, entity: EntityId) -> bool {
        if entity != self.agent {
            return false;
        }
        self.world
            .get_component_agent_data(entity)
            .is_some_and(|agent_data| agent_data.control_source != ControlSource::None)
    }
}

impl BelievedAuthorityView for PerAgentBeliefView<'_> {
    fn believed_owner_of(&self, entity: EntityId) -> BeliefRead<EntityId> {
        self.believed_entity_authority_claim(entity, EntityBeliefAspect::Owner)
    }

    fn believed_holder_of(&self, entity: EntityId) -> BeliefRead<EntityId> {
        self.believed_entity_authority_claim(entity, EntityBeliefAspect::Holder)
    }

    fn believed_office_holder(&self, office: EntityId) -> BeliefRead<Option<EntityId>> {
        let InstitutionalBeliefRead::Certain(holder) =
            self.belief_store.believed_office_holder(office)
        else {
            return BeliefRead::Unknown;
        };

        let learned_tick = self
            .belief_store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .into_iter()
            .flatten()
            .find_map(|belief| match belief.claim {
                worldwake_core::InstitutionalClaim::OfficeHolder {
                    office: claim_office,
                    holder: claim_holder,
                    effective_tick,
                } if claim_office == office && claim_holder == holder => {
                    Some((belief.learned_tick, effective_tick))
                }
                _ => None,
            })
            .unwrap_or((Tick(0), Tick(0)));

        BeliefRead::Known(crate::belief_view::BeliefValue {
            value: holder,
            confidence: Permille::new(1000).unwrap(),
            acquired_tick: learned_tick.0,
            claimed_event_tick: Some(learned_tick.1),
            status: crate::belief_view::BeliefStatus::Certain,
        })
    }
}

impl LocalPhysicalObservationView for PerAgentBeliefView<'_> {
    fn colocated_entities(&self, actor: EntityId) -> ObservedRead<Vec<EntityId>> {
        let value = if actor == self.agent {
            self.world
                .effective_place(actor)
                .map(|place| {
                    let mut entities = self.world.entities_effectively_at(place);
                    entities.sort();
                    entities.dedup();
                    entities
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        ObservedRead {
            value,
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_item_lot_quantity(&self, lot: EntityId) -> ObservedRead<Option<Quantity>> {
        ObservedRead {
            value: self
                .has_authoritative_local_visibility(lot)
                .then(|| {
                    self.world
                        .get_component_item_lot(lot)
                        .map(|lot| lot.quantity)
                })
                .flatten(),
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_workstation_tag(&self, entity: EntityId) -> ObservedRead<Option<WorkstationTag>> {
        ObservedRead {
            value: self
                .has_authoritative_local_visibility(entity)
                .then(|| {
                    self.world
                        .get_component_workstation_marker(entity)
                        .map(|marker| marker.0)
                })
                .flatten(),
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_resource_source(&self, entity: EntityId) -> ObservedRead<Option<ResourceSource>> {
        ObservedRead {
            value: self
                .has_authoritative_local_visibility(entity)
                .then(|| self.world.get_component_resource_source(entity).cloned())
                .flatten(),
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_container_contents(&self, container: EntityId) -> ObservedRead<Vec<EntityId>> {
        let value = if self.has_authoritative_local_visibility(container) {
            let mut contents = self.world.direct_contents_of(container);
            contents.sort();
            contents.dedup();
            contents
        } else {
            Vec::new()
        };

        ObservedRead {
            value,
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_entity_kind(&self, entity: EntityId) -> ObservedRead<Option<EntityKind>> {
        ObservedRead {
            value: self
                .has_authoritative_local_visibility(entity)
                .then(|| self.world.entity_kind(entity))
                .flatten(),
            observed_tick: self.current_tick,
            source: ObservationSource::CoLocatedSameTick,
        }
    }
}

impl EntityBeliefView for PerAgentBeliefView<'_> {
    fn is_alive(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            return self.world.is_alive(entity);
        }

        self.believed_entity(entity)
            .is_some_and(|state| state.alive)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        if self.world.topology().place(entity).is_some() {
            return Some(EntityKind::Place);
        }

        if entity == self.agent
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent)
        {
            return self.world.entity_kind(entity);
        }

        self.believed_entity(entity)
            .and_then(|belief| belief.believed_kind)
            .or_else(|| {
                self.world
                    .get_component_last_seen_memory(self.agent)
                    .and_then(|memory| memory.records.get(&entity))
                    .and_then(|record| record.observed_kind)
            })
    }

    fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        if !self.bandit_factions_of(self.agent).contains(&faction) {
            return None;
        }

        self.world
            .get_component_bandit_faction_policy(faction)
            .map(|policy| policy.flee_wound_threshold)
    }

    fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<NonZeroU32> {
        if !self.bandit_factions_of(self.agent).contains(&faction) {
            return None;
        }

        self.world
            .get_component_bandit_faction_policy(faction)
            .map(|policy| policy.establishment_duration_ticks)
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

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn believed_target_location(
        &self,
        agent: EntityId,
        target: EntityId,
    ) -> crate::belief_view::BeliefValue<Option<EntityId>> {
        if agent != self.agent {
            return crate::belief_view::stale_default_value(None);
        }

        crate::belief_view::project_claims_into_belief_set(
            self.belief_store
                .get_entity_claims(&target)
                .into_iter()
                .flatten()
                .filter_map(|claim| {
                    crate::belief_view::location_claim_value(claim)
                        .map(|value| (claim.clone(), value))
                }),
            self.current_tick,
            self.claim_confidence_threshold(agent),
            &self.belief_confidence_policy(agent),
        )
        .best
        .unwrap_or_else(|| crate::belief_view::stale_default_value(None))
    }
}

impl ProfileBeliefView for PerAgentBeliefView<'_> {
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

    fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_deprivation_exposure(agent)
                    .copied()
            })
            .flatten()
    }

    fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_drive_escalation_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_metabolism_profile(agent).copied())
            .flatten()
    }

    fn testimony_trust_profile(&self, agent: EntityId) -> Option<TestimonyTrustProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_testimony_trust_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn route_preference_profile(&self, agent: EntityId) -> Option<RoutePreferenceProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_route_preference_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile {
        if agent != self.agent {
            return SleepQualityProfile::default();
        }

        let place_is_known =
            self.world.effective_place(agent) == Some(place) || self.knows_entity(place);
        if !place_is_known {
            return SleepQualityProfile::default();
        }

        self.world
            .get_component_sleep_quality_profile(place)
            .copied()
            .unwrap_or_default()
    }

    fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness {
        if agent != self.agent {
            return PlaceDirtiness::default();
        }

        if self.world.effective_place(agent) != Some(place) {
            return PlaceDirtiness::default();
        }

        self.world
            .get_component_place_dirtiness(place)
            .copied()
            .unwrap_or_default()
    }

    fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
        if agent != self.agent {
            return LatrineFullness::default();
        }

        if self.world.effective_place(agent) != Some(place) {
            return LatrineFullness::default();
        }

        self.world
            .get_component_latrine_fullness(place)
            .copied()
            .unwrap_or_default()
    }

    fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
        if agent != self.agent {
            return WashBasinState::default();
        }
        if self.has_authoritative_local_visibility(basin) {
            return self
                .world
                .get_component_wash_basin_state(basin)
                .copied()
                .unwrap_or_default();
        }
        // FND-14A: physical state requires co-location to observe; off-place
        // ranking falls through to the agent's most recent stored belief
        // (`BelievedEntityState::wash_basin_state`) rather than reading
        // authoritative state for a non-co-located entity.
        self.believed_entity(basin)
            .and_then(|state| state.wash_basin_state)
            .unwrap_or_default()
    }

    fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_disposal_profile(agent).copied())
            .flatten()
    }

    fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_artifact_posting_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn exploration_profile(&self, agent: EntityId) -> Option<worldwake_core::ExplorationProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_exploration_profile(agent).copied())
            .flatten()
    }

    fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_diversification_profile(agent)
                    .copied()
            })
            .flatten()
    }

    fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_last_proactive_exploration_tick(agent)
                    .copied()
            })
            .flatten()
            .and_then(|LastProactiveExplorationTick(tick)| tick)
    }

    fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
        if agent != self.agent {
            return 0;
        }

        self.world
            .get_component_acquisition_exhaustion_tracker(agent)
            .map_or(0, |tracker| tracker.count(need))
    }

    fn obligation_satiation_profile(&self, agent: EntityId) -> ObligationSatiationProfile {
        if agent != self.agent {
            return ObligationSatiationProfile::default();
        }

        self.world
            .get_component_obligation_satiation_profile(agent)
            .cloned()
            .unwrap_or_default()
    }

    fn obligation_execution_tracker(&self, agent: EntityId) -> ObligationExecutionTracker {
        if agent != self.agent {
            return ObligationExecutionTracker::default();
        }

        self.world
            .get_component_obligation_execution_tracker(agent)
            .cloned()
            .unwrap_or_default()
    }

    fn cognitive_profile(&self, agent: EntityId) -> Option<CognitiveProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_cognitive_profile(agent).copied())
            .flatten()
    }

    fn portfolio_weights_profile(&self, agent: EntityId) -> PortfolioWeightsProfile {
        if agent != self.agent {
            return PortfolioWeightsProfile::default();
        }

        self.world
            .get_component_portfolio_weights_profile(agent)
            .copied()
            .unwrap_or_default()
    }

    fn agent_schema_context_profile(&self, agent: EntityId) -> Option<AgentSchemaContextProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_agent_schema_context_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn perception_profile(&self, agent: EntityId) -> Option<PerceptionProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_perception_profile(agent).copied())
            .flatten()
    }

    fn risk_weight_profile(&self, agent: EntityId) -> Option<RiskWeightProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_risk_weight_profile(agent).copied())
            .flatten()
    }

    fn law_abiding_profile(&self, agent: EntityId) -> Option<LawAbidingProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_law_abiding_profile(agent).copied())
            .flatten()
    }

    fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_preference_profile(agent).copied())
            .flatten()
    }

    fn utility_profile(&self, agent: EntityId) -> Option<UtilityProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_utility_profile(agent).cloned())
            .flatten()
    }
}

impl SpatialBeliefView for PerAgentBeliefView<'_> {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        if entity == self.agent {
            return self.world.effective_place(entity);
        }

        if self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent)
        {
            return self.world.effective_place(entity);
        }

        self.believed_entity(entity)
            .and_then(|state| state.last_known_place)
            .or_else(|| {
                self.world
                    .get_component_last_seen_memory(self.agent)
                    .and_then(|memory| memory.records.get(&entity).map(|record| record.place))
            })
    }

    fn is_in_transit(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            return self.world.is_in_transit(entity);
        }

        false
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        if self.world.effective_place(self.agent) == Some(place) {
            let mut entities = self.world.entities_effectively_at(place);
            entities.sort();
            entities.dedup();
            return entities;
        }

        let mut entities = self
            .belief_store
            .known_entities
            .iter()
            .filter_map(|(entity, state)| {
                (state.last_known_place == Some(place)).then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.world.topology().neighbors(place)
    }

    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        self.world.place_has_tag(place, tag)
    }

    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        (agent == self.agent)
            .then(|| self.world.get_component_route_experience(agent).cloned())
            .flatten()
    }

    fn patrol_route(&self, agent: EntityId) -> Option<worldwake_core::PatrolRoute> {
        (agent == self.agent)
            .then(|| self.world.get_component_patrol_route(agent).cloned())
            .flatten()
    }

    fn office_patrol_duty(&self, agent: EntityId) -> Option<worldwake_core::OfficePatrolDuty> {
        (agent == self.agent)
            .then(|| self.world.get_component_office_patrol_duty(agent).cloned())
            .flatten()
    }

    fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
        self.world.topology().shortest_path(from, to).is_some()
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

    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<crate::belief_view::BeliefValue<EntityId>> {
        if agent != self.agent {
            return Vec::new();
        }

        let threshold = self.claim_confidence_threshold(agent);
        let policy = self.belief_confidence_policy(agent);

        self.belief_store
            .iter_known_entities()
            .filter_map(|(subject, state)| (state.believed_kind == Some(kind)).then_some(*subject))
            .filter_map(|subject| {
                let best = crate::belief_view::project_claims_into_belief_set(
                    self.belief_store
                        .get_entity_claims(&subject)
                        .into_iter()
                        .flatten()
                        .filter_map(|claim| {
                            crate::belief_view::location_claim_value(claim)
                                .map(|value| (claim.clone(), value))
                        }),
                    self.current_tick,
                    threshold,
                    &policy,
                )
                .best?;
                (best.value == Some(place)).then_some(crate::belief_view::BeliefValue {
                    value: subject,
                    confidence: best.confidence,
                    acquired_tick: best.acquired_tick,
                    claimed_event_tick: best.claimed_event_tick,
                    status: best.status,
                })
            })
            .collect()
    }
}

impl TemporalBeliefView for PerAgentBeliefView<'_> {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }

    fn has_contention_policy(&self, entity: EntityId) -> bool {
        self.world.get_component_contention_policy(entity).is_some()
    }

    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        if !self.has_authoritative_local_visibility(facility) {
            return None;
        }
        self.world
            .get_component_contention_queue(facility)
            .and_then(|queue| queue.position_of(actor))
    }

    fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
        if !self.has_authoritative_local_visibility(facility) {
            return None;
        }
        self.world
            .get_component_contention_queue(facility)
            .and_then(|queue| queue.granted.as_ref())
    }

    fn extraction_slot_queue_position(&self, source: EntityId, actor: EntityId) -> Option<u32> {
        if !self.has_authoritative_local_visibility(source) {
            return None;
        }
        self.world
            .get_component_resource_extraction_queues(source)
            .and_then(|queues| {
                queues
                    .queues
                    .iter()
                    .find_map(|queue| queue.position_of(actor))
            })
    }

    fn actor_holds_extraction_slot_grant(&self, source: EntityId, actor: EntityId) -> bool {
        if !self.has_authoritative_local_visibility(source) {
            return false;
        }
        self.world
            .get_component_resource_extraction_queues(source)
            .is_some_and(|queues| {
                queues.queues.iter().any(|queue| {
                    queue
                        .granted
                        .as_ref()
                        .is_some_and(|grant| grant.actor == actor)
                })
            })
    }

    fn actor_can_claim_extraction_slot(&self, source: EntityId, actor: EntityId) -> bool {
        if !self.has_authoritative_local_visibility(source) {
            return false;
        }
        self.world
            .get_component_resource_extraction_queues(source)
            .is_some_and(|queues| {
                queues.queues.iter().any(|queue| match &queue.granted {
                    Some(_) => false,
                    None => match queue.waiting.values().next() {
                        Some(head) => head.actor == actor,
                        None => true,
                    },
                })
            })
    }

    fn has_extraction_queues(&self, source: EntityId) -> bool {
        if !self.has_authoritative_local_visibility(source) {
            return false;
        }
        self.world
            .get_component_resource_extraction_queues(source)
            .is_some_and(|queues| !queues.queues.is_empty())
    }

    fn contention_queue_is_full(&self, entity: EntityId) -> bool {
        if !self.has_authoritative_local_visibility(entity) {
            let Some(contention) = self.believed_contention_state_of(entity) else {
                return false;
            };
            let Some(policy) = self.world.get_component_contention_policy(entity) else {
                return false;
            };
            return policy
                .max_waiters
                .is_some_and(|limit| contention.queue_length >= u32::from(limit));
        }
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
        if !self.has_authoritative_local_visibility(facility) {
            return None;
        }
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

    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
        if !self.has_authoritative_local_visibility(entity) {
            return false;
        }
        self.world
            .reservations_for(entity)
            .into_iter()
            .any(|reservation| reservation.range.overlaps(&range))
    }

    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
        if !self.has_authoritative_local_visibility(entity) {
            return Vec::new();
        }
        self.world
            .reservations_for(entity)
            .into_iter()
            .map(|reservation| reservation.range)
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

impl SocialBeliefView for PerAgentBeliefView<'_> {
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        if agent != self.agent {
            return Vec::new();
        }

        let mut beliefs = self
            .belief_store
            .known_entities
            .iter()
            .map(|(entity, state)| (*entity, state.clone()))
            .collect::<Vec<_>>();

        if let Some(memory) = self.world.get_component_last_seen_memory(agent) {
            for (entity, record) in &memory.records {
                if beliefs.iter().any(|(known, _)| known == entity) {
                    continue;
                }
                let source = match record.provenance {
                    LastSeenProvenance::DirectObservation => PerceptionSource::DirectObservation,
                    LastSeenProvenance::Hearsay { chain_depth, .. } => PerceptionSource::Report {
                        from: record.source,
                        chain_len: chain_depth,
                    },
                };
                beliefs.push((
                    *entity,
                    BelievedEntityState {
                        believed_kind: record.observed_kind,
                        last_known_place: Some(record.place),
                        alive: true,
                        ..BelievedEntityState::single_observation_defaults(
                            record.observed_tick,
                            source,
                        )
                    },
                ));
            }
        }

        beliefs
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

    fn claim_confidence_threshold(&self, agent: EntityId) -> Permille {
        assert_eq!(
            agent, self.agent,
            "claim_confidence_threshold is a self-authoritative read and must only be requested for the acting agent"
        );
        self.world
            .get_component_perception_profile(agent)
            .map(|profile| profile.claim_confidence_threshold)
            .expect(
                "acting agents must have PerceptionProfile before reading claim confidence threshold",
            )
    }

    fn discrepancy_memory(&self, agent: EntityId) -> Option<&worldwake_core::DiscrepancyMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_discrepancy_memory(agent))
            .flatten()
    }

    fn blocker_memory(&self, agent: EntityId) -> Option<&worldwake_core::BlockerMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_blocker_memory(agent))
            .flatten()
    }

    fn repair_memory(&self, agent: EntityId) -> Option<&worldwake_core::RepairMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_repair_memory(agent))
            .flatten()
    }

    fn learned_opportunity_memory(
        &self,
        agent: EntityId,
    ) -> Option<&worldwake_core::LearnedOpportunityMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_learned_opportunity_memory(agent))
            .flatten()
    }

    fn survey_memory(&self, agent: EntityId) -> Option<&SurveyMemory> {
        (agent == self.agent)
            .then(|| self.world.get_component_survey_memory(agent))
            .flatten()
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

    fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
        (agent == self.agent)
            .then(|| self.world.get_component_source_reliability(agent).cloned())
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
}

impl PoliticalBeliefView for PerAgentBeliefView<'_> {
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

    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_justice_disposition_profile(agent)
                    .cloned()
            })
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

    fn record_data(&self, record: EntityId) -> Option<worldwake_core::RecordData> {
        self.belief_store
            .believed_record_data(record)
            .map(|snapshot| snapshot.data.clone())
    }

    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        self.belief_store
            .believed_office_data(office)
            .map(|snapshot| snapshot.data.clone())
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
        self.believed_entity(target)?;

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

    fn visible_reward_encumbrance(
        &self,
        actor: EntityId,
        office: EntityId,
    ) -> Option<&RewardEncumbrance> {
        if actor != self.agent
            || !matches!(
                BelievedAuthorityView::believed_office_holder(self, office),
                BeliefRead::Known(value) | BeliefRead::Stale(value) if value.value == Some(actor)
            )
        {
            return None;
        }

        self.world.get_component_reward_encumbrance(office)
    }
}

impl RuntimeBeliefView for PerAgentBeliefView<'_> {}

impl InventoryBeliefView for PerAgentBeliefView<'_> {
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        if holder == self.agent {
            return self.world.possessions_of(holder);
        }

        Vec::new()
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
        if self.world.possessor_of(holder) == Some(self.agent) {
            return self
                .world
                .get_component_item_lot(holder)
                .filter(|lot| lot.commodity == kind)
                .map_or(Quantity(0), |lot| lot.quantity);
        }
        if self.has_authoritative_local_visibility(holder) {
            if let Some(lot) = self.world.get_component_item_lot(holder)
                && lot.commodity == kind
            {
                return lot.quantity;
            }
            if let Some(source) = self.world.get_component_resource_source(holder)
                && source.commodity == kind
            {
                return source.available_quantity;
            }
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

    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        let accessible = self.knows_entity(entity)
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent);
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
        let accessible = self.knows_entity(entity)
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent);
        accessible
            .then(|| self.world.direct_container(entity))
            .flatten()
    }

    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        let accessible = self.has_authoritative_local_visibility(entity)
            || self.knows_entity(entity)
            || self.world.possessor_of(entity) == Some(self.agent);
        accessible
            .then(|| self.world.possessor_of(entity))
            .flatten()
    }

    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
        let observable = entity == self.agent
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent);
        observable
            .then(|| {
                self.world
                    .get_component_carry_capacity(entity)
                    .map(|CarryCapacity(capacity)| *capacity)
            })
            .flatten()
    }

    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
        let observable = entity == self.agent
            || self.has_authoritative_local_visibility(entity)
            || self.world.possessor_of(entity) == Some(self.agent);
        observable
            .then(|| load_of_entity(self.world, entity).ok())
            .flatten()
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

    fn believed_commodity_stock(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: CommodityKind,
    ) -> crate::belief_view::BeliefValue<Quantity> {
        if agent != self.agent {
            return crate::belief_view::stale_default_value(Quantity(0));
        }

        crate::belief_view::project_claims_into_belief_set(
            self.belief_store
                .get_entity_claims(&place)
                .into_iter()
                .flatten()
                .filter_map(|claim| {
                    crate::belief_view::inventory_claim_value(claim, kind)
                        .map(|value| (claim.clone(), value))
                }),
            self.current_tick,
            self.claim_confidence_threshold(agent),
            &self.belief_confidence_policy(agent),
        )
        .best
        .unwrap_or_else(|| crate::belief_view::stale_default_value(Quantity(0)))
    }
}

impl CombatBeliefView for PerAgentBeliefView<'_> {
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

    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        if agent != self.agent {
            return Vec::new();
        }

        self.world
            .hostile_targets_of(agent)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| {
                self.believed_entity(*entity)
                    .is_some_and(|belief| belief.alive)
                    || self
                        .world
                        .get_component_last_seen_memory(agent)
                        .is_some_and(|memory| memory.records.contains_key(entity))
            })
            .collect()
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

    fn patrol_profile(&self, agent: EntityId) -> Option<worldwake_core::PatrolProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_patrol_profile(agent).cloned())
            .flatten()
    }

    fn pursuit_profile(&self, agent: EntityId) -> Option<worldwake_core::PursuitProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_pursuit_profile(agent).cloned())
            .flatten()
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
}

impl EconomicBeliefView for PerAgentBeliefView<'_> {
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

    fn substitute_preferences(&self, agent: EntityId) -> Option<SubstitutePreferences> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_substitute_preferences(agent)
                    .cloned()
            })
            .flatten()
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

    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        if self.world.effective_place(self.agent) != Some(place) {
            return Vec::new();
        }

        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::ItemLot))
            .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
            .filter(|entity| self.world.has_component_sale_listing(*entity))
            .filter(|entity| {
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
        if !self.has_authoritative_local_visibility(lot) {
            return None;
        }

        if !self.world.has_component_sale_listing(lot) {
            return None;
        }
        let assignment = self.world.get_component_stock_assignment(lot)?;
        if assignment.kind != worldwake_core::StockAssignmentKind::Displayed {
            return None;
        }
        let place = self.effective_place(lot)?;
        self.facility_controller_at(assignment.facility, place)
    }

    fn has_sale_listing(&self, lot: EntityId) -> bool {
        self.has_authoritative_local_visibility(lot) && self.world.has_component_sale_listing(lot)
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
}

impl FacilityBeliefView for PerAgentBeliefView<'_> {
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self
                .world
                .get_component_workstation_marker(entity)
                .map(|marker| marker.0);
        }

        self.believed_entity(entity)
            .and_then(|state| state.workstation_tag)
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self.world.get_component_resource_source(entity).cloned();
        }

        self.believed_entity(entity)
            .and_then(|state| state.resource_source.clone())
    }

    fn wash_basin_state(&self, entity: EntityId) -> Option<WashBasinState> {
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self.world.get_component_wash_basin_state(entity).copied();
        }
        // FND-14A: off-place reads consult the most recent stored belief
        // about the basin's state. The authoritative wash precondition
        // re-validates live state at action time, so stale beliefs trigger
        // replan rather than commit-against-stale-state.
        self.believed_entity(entity)
            .and_then(|state| state.wash_basin_state)
    }

    fn last_harvest_trace(&self, entity: EntityId) -> Option<LastHarvestTrace> {
        // FND-14A: only co-located agents can perceive a source's harvest
        // trace directly. Off-place propagation is the responsibility of
        // ShareBelief / report channels (not this surface).
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self.world.get_component_last_harvest_trace(entity).cloned();
        }
        None
    }

    fn resource_extraction_queues(&self, entity: EntityId) -> Option<ResourceExtractionQueues> {
        // FND-14A: extraction queues are physical occupancy of the source
        // (slot count and per-slot queue/grant state are concrete world
        // state on a co-located entity). Off-place propagation goes
        // through ShareBelief / report channels, not this surface.
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self
                .world
                .get_component_resource_extraction_queues(entity)
                .cloned();
        }
        None
    }

    fn has_production_job(&self, entity: EntityId) -> bool {
        if entity == self.agent || self.has_authoritative_local_visibility(entity) {
            return self.world.has_component_production_job(entity);
        }
        self.believed_activity_of(entity).is_some_and(|activity| {
            activity.action_domain == worldwake_core::ActionDomain::Production
        })
    }

    fn stock_storage_policy(&self, facility: EntityId) -> Option<StockStoragePolicy> {
        self.believed_entity(facility).and_then(|_| {
            self.world
                .get_component_stock_storage_policy(facility)
                .cloned()
        })
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
}

#[cfg(test)]
mod tests {
    use super::{PerAgentBeliefRuntime, PerAgentBeliefView};
    use crate::{
        ActionDef, ActionDefRegistry, ActionDuration, ActionHandlerId, ActionInstance,
        ActionInstanceId, ActionPayload, ActionStatus, BeliefRead, BelievedAuthorityView,
        CombatBeliefView, Constraint, ControlBeliefView, DebugWorldView, DurationExpr,
        EconomicBeliefView, EntityBeliefView, FacilityBeliefView, GoalBeliefView, Interruptibility,
        InventoryBeliefView, LocalPhysicalObservationView, ObservationSource, Precondition,
        ProfileBeliefView, ReservationReq, RuntimeBeliefView, SpatialBeliefView, TargetSpec,
        TemporalBeliefView,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, AgentBeliefStore, AgentSchemaContextProfile,
        ArtifactPostingProfile, BanditFactionPolicy, BeliefConfidencePolicy, BelievedEntityState,
        BodyCostPerTick, BodyPart, CandidateExtractorId, CauseRef, ClaimId, ClaimValue,
        CognitiveProfile, CombatProfile, CommodityKind, ContentionQueue, ContentionWaiter,
        ControlSource, DisposalProfile, EdgeExperience, EffectiveRight, EntityBeliefAspect,
        EntityBeliefClaim, EntityId, EntityKind, EntityState, EventLog, ExpectationBasis,
        ExpectationId, ExpectationRecord, ExpectationState, ExpectationStore, ExplorationProfile,
        FactionData, FactionPurpose, GoalDispatchKey, GoalPlanningBudget, HarvestTraceEntry,
        HomeostaticNeedId, InstitutionalBeliefKey, InstitutionalBeliefRead, InstitutionalClaim,
        InstitutionalKnowledgeSource, LastHarvestTrace, LastSeenMemory, LastSeenProvenance,
        LastSeenRecord, LawAbidingProfile, LoadUnits, ObligationExecutionTracker,
        ObligationSatiationProfile, OfficeData, PerceptionProfile, PerceptionSource, Permille,
        Place, PlaceTag, PreferenceProfile, Quantity, RecipientKnowledgeStatus, RecordData,
        RecordKind, ResourceExtractionQueues, ResourceSource, RightKind, RiskWeightProfile,
        RouteExperience, StockStoragePolicy, SuccessionLaw, TellMemoryKey, TellTopic, Tick,
        TickRange, ToldBeliefMemory, Topology, TravelEdge, TravelEdgeId, UtilityProfile,
        VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, Wound,
        WoundCause, WoundId, build_believed_entity_state, build_prototype_world,
        test_utils::{
            sample_blocker_memory, sample_commodity_valuation_profile, sample_discrepancy_memory,
            sample_learned_opportunity_memory, sample_preference_profile, sample_repair_memory,
            sample_route_experience, sample_source_reliability,
        },
    };

    fn assert_goal_belief_view<T: GoalBeliefView>() {}
    fn assert_runtime_belief_view<T: RuntimeBeliefView>() {}

    #[test]
    fn agent_schema_context_profile_returns_seeded_profile() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 0);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };
        let mut profile = AgentSchemaContextProfile::default();
        profile
            .disabled_extractors
            .insert(CandidateExtractorId::Disposal);
        profile.budget_overrides.insert(
            GoalDispatchKey::ProduceCommodity,
            GoalPlanningBudget::PRODUCTION,
        );
        {
            let mut txn = new_txn(&mut world, 1);
            txn.set_component_agent_schema_context_profile(agent, profile.clone())
                .unwrap();
            commit_txn(txn);
        }
        let beliefs = world.get_component_agent_belief_store(agent).unwrap();
        let view = PerAgentBeliefView::new(agent, &world, beliefs);

        assert_eq!(
            ProfileBeliefView::agent_schema_context_profile(&view, agent),
            Some(profile)
        );
    }

    fn entity_belief(
        place: worldwake_core::EntityId,
        alive: bool,
        bread: u32,
        observed_tick: u64,
    ) -> BelievedEntityState {
        let mut inventory = BTreeMap::new();
        inventory.insert(CommodityKind::Bread, Quantity(bread));
        BelievedEntityState {
            believed_kind: None,
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
            ..BelievedEntityState::single_observation_defaults(
                Tick(observed_tick),
                worldwake_core::PerceptionSource::DirectObservation,
            )
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
            binding_strictness: crate::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: crate::EffectSchema::empty(),
        }
    }

    fn sample_claim(
        claim_id: u64,
        subject: EntityId,
        aspect: EntityBeliefAspect,
        value: ClaimValue,
        acquired_tick: u64,
        confidence: u16,
    ) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(claim_id),
            subject,
            aspect,
            value,
            source: PerceptionSource::DirectObservation,
            acquired_tick: Tick(acquired_tick),
            claimed_event_tick: Some(Tick(acquired_tick)),
            confidence: Permille::new(confidence).unwrap(),
            refuted_at_tick: None,
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
                            observed_kind: Some(EntityKind::Agent),
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
                observed_kind: Some(EntityKind::Agent),
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
        let authoritative_other_place = places[2];
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, authoritative_other_place)
                .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, entity_belief(believed_place, false, 7, 10));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::homeostatic_needs(&view, agent),
            world.get_component_homeostatic_needs(agent).copied()
        );
        assert_eq!(
            SpatialBeliefView::effective_place(&view, agent),
            Some(place)
        );
        assert_eq!(
            InventoryBeliefView::commodity_quantity(&view, agent, CommodityKind::Bread),
            world.controlled_commodity_quantity(agent, CommodityKind::Bread)
        );
        assert_eq!(
            SpatialBeliefView::effective_place(&view, other),
            Some(believed_place)
        );
        assert!(!EntityBeliefView::is_alive(&view, other));
        assert!(EntityBeliefView::is_dead(&view, other));
        assert_eq!(
            InventoryBeliefView::commodity_quantity(&view, other, CommodityKind::Bread),
            Quantity(7)
        );
        assert_eq!(CombatBeliefView::wounds(&view, other), vec![sample_wound()]);
    }

    #[test]
    fn directly_possessed_item_lot_quantity_uses_authoritative_quantity_over_stale_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Waste, Quantity(6))
                .unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_possessor(lot, agent).unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        let mut stale_lot_belief = entity_belief(place, true, 0, 10);
        stale_lot_belief.believed_kind = Some(EntityKind::ItemLot);
        stale_lot_belief.last_known_inventory.clear();
        stale_lot_belief
            .last_known_inventory
            .insert(CommodityKind::Waste, Quantity(1));
        beliefs.update_entity(lot, stale_lot_belief);

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            InventoryBeliefView::commodity_quantity(&view, lot, CommodityKind::Waste),
            Quantity(6)
        );
    }

    #[test]
    fn current_place_entities_use_authoritative_local_set_over_stale_beliefs() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let remote_place = world
            .topology()
            .place_ids()
            .find(|candidate| *candidate != place)
            .expect("prototype world should include a second place");
        let (agent, stale_lot, visible_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let stale_lot = txn
                .create_item_lot(CommodityKind::Apple, Quantity(2))
                .unwrap();
            txn.set_ground_location(stale_lot, remote_place).unwrap();
            let visible_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(visible_lot, place).unwrap();
            commit_txn(txn);
            (agent, stale_lot, visible_lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        let mut stale_lot_belief = entity_belief(place, true, 0, 10);
        stale_lot_belief.believed_kind = Some(EntityKind::ItemLot);
        stale_lot_belief.last_known_inventory.clear();
        stale_lot_belief
            .last_known_inventory
            .insert(CommodityKind::Apple, Quantity(2));
        beliefs.update_entity(stale_lot, stale_lot_belief);

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let entities = SpatialBeliefView::entities_at(&view, place);

        assert!(
            entities.contains(&agent),
            "agent should still observe itself at the current place"
        );
        assert!(
            entities.contains(&visible_lot),
            "authoritative local entities should stay visible at the current place"
        );
        assert!(
            !entities.contains(&stale_lot),
            "stale believed current-place lots must not remain visible once the actor is co-located"
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
            EconomicBeliefView::listed_sale_lots_at(&view, place, CommodityKind::Bread),
            vec![listed_lot]
        );
        assert_eq!(
            EconomicBeliefView::seller_for_sale_lot(&view, listed_lot),
            Some(believed_merchant)
        );
        // Hidden lot's seller is not discoverable (agent doesn't know about hidden_lot)
        assert_eq!(
            EconomicBeliefView::seller_for_sale_lot(&view, hidden_lot),
            None
        );
    }

    #[test]
    fn remote_listed_sale_lot_does_not_read_live_sale_listing() {
        use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};

        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let home = places[0];
        let market = places[1];
        let (agent, merchant, display, listed_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let merchant = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, home).unwrap();
            txn.set_ground_location(merchant, market).unwrap();

            let (facility, _stock, display) = txn
                .create_merchant_facility(market, merchant, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display = display.expect("merchant facility should include a display container");
            let listed_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(5))
                .unwrap();
            txn.put_into_container(listed_lot, display).unwrap();
            txn.set_component_stock_assignment(
                listed_lot,
                StockAssignment {
                    facility,
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

            commit_txn(txn);
            (agent, merchant, display, listed_lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(merchant, entity_belief(market, true, 3, 5));
        beliefs.update_entity(listed_lot, entity_belief(market, true, 3, 5));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            EconomicBeliefView::listed_sale_lots_at(&view, market, CommodityKind::Bread),
            Vec::<EntityId>::new()
        );
        assert_eq!(
            EconomicBeliefView::seller_for_sale_lot(&view, listed_lot),
            None
        );
        assert!(!EconomicBeliefView::has_sale_listing(&view, listed_lot));
        assert_eq!(
            InventoryBeliefView::direct_container(&view, listed_lot),
            Some(display)
        );
        assert_eq!(
            InventoryBeliefView::direct_possessor(&view, listed_lot),
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
            SpatialBeliefView::effective_place(&view, other),
            Some(place_a)
        );
    }

    #[test]
    fn effective_place_uses_last_seen_without_refreshing_remote_truth() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let last_seen_place = places[1];
        let current_remote_place = places[2];
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, actor_place).unwrap();
            txn.set_ground_location(other, last_seen_place).unwrap();
            txn.set_component_last_seen_memory(
                agent,
                LastSeenMemory {
                    records: BTreeMap::from([(
                        other,
                        LastSeenRecord {
                            subject: other,
                            place: last_seen_place,
                            observed_kind: Some(EntityKind::Agent),
                            observed_tick: Tick(1),
                            source: agent,
                            provenance: LastSeenProvenance::DirectObservation,
                        },
                    )]),
                    capacity: 8,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, other)
        };
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(other, current_remote_place)
                .unwrap();
            commit_txn(txn);
        }

        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(world.effective_place(other), Some(current_remote_place));
        assert_eq!(
            SpatialBeliefView::effective_place(&view, other),
            Some(last_seen_place)
        );
    }

    #[test]
    fn effective_place_keeps_authoritative_reads_for_local_or_possessed_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let remote_place = places[1];
        let (agent, co_located, possessed) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let co_located = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let possessed = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(agent, actor_place).unwrap();
            txn.set_ground_location(co_located, actor_place).unwrap();
            txn.set_ground_location(possessed, remote_place).unwrap();
            txn.set_possessor(possessed, agent).unwrap();
            commit_txn(txn);
            (agent, co_located, possessed)
        };

        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(
            SpatialBeliefView::effective_place(&view, co_located),
            Some(actor_place)
        );
        assert_eq!(
            SpatialBeliefView::effective_place(&view, possessed),
            world.effective_place(possessed)
        );
    }

    #[test]
    fn entity_kind_uses_stored_belief_for_remote_entity_not_live_world() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let remote_place = places[1];
        let (agent, remote_facility) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, actor_place).unwrap();
            let remote_facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(remote_facility, remote_place)
                .unwrap();
            commit_txn(txn);
            (agent, remote_facility)
        };

        let mut beliefs = AgentBeliefStore::new();
        let mut stale_belief = entity_belief(remote_place, true, 0, 1);
        stale_belief.believed_kind = Some(EntityKind::Agent);
        beliefs.update_entity(remote_facility, stale_belief);
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            world.entity_kind(remote_facility),
            Some(EntityKind::Facility)
        );
        assert_eq!(
            EntityBeliefView::entity_kind(&view, remote_facility),
            Some(EntityKind::Agent)
        );
    }

    #[test]
    fn entity_kind_uses_last_seen_kind_for_remote_entity_without_belief_store_entry() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let remote_place = places[1];
        let (agent, remote_facility) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, actor_place).unwrap();
            let remote_facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(remote_facility, remote_place)
                .unwrap();
            txn.set_component_last_seen_memory(
                agent,
                LastSeenMemory {
                    records: BTreeMap::from([(
                        remote_facility,
                        LastSeenRecord {
                            subject: remote_facility,
                            place: remote_place,
                            observed_kind: Some(EntityKind::Agent),
                            observed_tick: Tick(1),
                            source: agent,
                            provenance: LastSeenProvenance::DirectObservation,
                        },
                    )]),
                    capacity: 8,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, remote_facility)
        };

        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(
            world.entity_kind(remote_facility),
            Some(EntityKind::Facility)
        );
        assert_eq!(
            EntityBeliefView::entity_kind(&view, remote_facility),
            Some(EntityKind::Agent)
        );
    }

    #[test]
    fn entity_kind_keeps_live_read_for_co_located_entity_without_stored_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, local_facility) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let local_facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(local_facility, place).unwrap();
            commit_txn(txn);
            (agent, local_facility)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            EntityBeliefView::entity_kind(&view, local_facility),
            Some(EntityKind::Facility)
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
            crate::SocialBeliefView::known_entity_beliefs(&view, agent),
            vec![(other, entity_belief(place, true, 2, 4))]
        );
        assert!(
            crate::SocialBeliefView::known_entity_beliefs(&view, other).is_empty(),
            "belief enumeration should not expose another agent's store through this view"
        );
    }

    #[test]
    fn known_entity_beliefs_synthesize_last_seen_kind_from_record_not_live_world() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let last_seen_place = places[1];
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, actor_place).unwrap();
            txn.set_ground_location(other, last_seen_place).unwrap();
            txn.set_component_last_seen_memory(
                agent,
                LastSeenMemory {
                    records: BTreeMap::from([(
                        other,
                        LastSeenRecord {
                            subject: other,
                            place: last_seen_place,
                            observed_kind: Some(EntityKind::ItemLot),
                            observed_tick: Tick(1),
                            source: agent,
                            provenance: LastSeenProvenance::DirectObservation,
                        },
                    )]),
                    capacity: 8,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let view = PerAgentBeliefView::from_world(agent, &world);
        let beliefs = crate::SocialBeliefView::known_entity_beliefs(&view, agent);

        assert_eq!(world.entity_kind(other), Some(EntityKind::Agent));
        assert_eq!(beliefs.len(), 1);
        assert_eq!(beliefs[0].0, other);
        assert_eq!(beliefs[0].1.believed_kind, Some(EntityKind::ItemLot));
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
            crate::SocialBeliefView::believed_activity_of(&view, other),
            Some(&worldwake_core::BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(entity(40)),
                observed_tick: Tick(8),
            })
        );
        assert_eq!(
            crate::SocialBeliefView::believed_activity_of(&view, agent),
            None
        );
        assert_eq!(
            crate::SocialBeliefView::believed_activity_of(&view, unknown),
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
            crate::SocialBeliefView::agents_active_at(&view, place, ActionDomain::Production, None),
            vec![a, c]
        );
        assert_eq!(
            crate::SocialBeliefView::agents_active_at(
                &view,
                place,
                ActionDomain::Production,
                Some(source)
            ),
            vec![a]
        );
        assert_eq!(
            crate::SocialBeliefView::agents_active_at(
                &view,
                place,
                ActionDomain::Trade,
                Some(source)
            ),
            vec![b]
        );
        assert!(
            crate::SocialBeliefView::agents_active_at(
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
            crate::SocialBeliefView::told_belief_memory(&view, agent, listener, &topic)
                .map(|m| m.told_tick),
            Some(Tick(4))
        );
        assert_eq!(
            crate::SocialBeliefView::recipient_knowledge_status(&view, agent, listener, &topic),
            Some(RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief)
        );

        let expired_view = PerAgentBeliefView::new_at_tick(agent, Tick(60), &world, &beliefs);
        assert_eq!(
            crate::SocialBeliefView::told_belief_memory(&expired_view, agent, listener, &topic),
            None
        );
        assert_eq!(
            crate::SocialBeliefView::recipient_knowledge_status(
                &expired_view,
                agent,
                listener,
                &topic,
            ),
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
            crate::SocialBeliefView::told_belief_memories(&view, agent).len(),
            1
        );
        assert!(
            crate::SocialBeliefView::told_belief_memories(&view, other).is_empty(),
            "conversation memory should remain actor-local"
        );
        assert_eq!(
            crate::SocialBeliefView::told_belief_memory(&view, other, listener, &topic),
            None
        );
        assert_eq!(
            crate::SocialBeliefView::recipient_knowledge_status(&view, other, listener, &topic),
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

        assert_eq!(crate::SocialBeliefView::tell_profile(&view, agent), None);
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
            EconomicBeliefView::commodity_valuation_profile(&view, agent),
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
            EconomicBeliefView::commodity_valuation_profile(&view, agent),
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
            SpatialBeliefView::route_experience(&view, agent),
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

        assert_eq!(SpatialBeliefView::route_experience(&view, agent), None);
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
            crate::SocialBeliefView::source_reliability(&view, agent),
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

        assert_eq!(
            crate::SocialBeliefView::source_reliability(&view, agent),
            None
        );
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
            ProfileBeliefView::preference_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::preference_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn disposal_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = DisposalProfile {
            capacity_strain_threshold: Permille::new(875).unwrap(),
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_disposal_profile(agent, profile).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::disposal_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::disposal_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn disposal_profile_returns_none_for_non_self_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(ProfileBeliefView::disposal_profile(&view, other), None);
        assert_eq!(GoalBeliefView::disposal_profile(&view, other), None);
    }

    #[test]
    fn artifact_posting_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = ArtifactPostingProfile {
            threat_warning_ttl: 36,
            office_vacancy_ttl: 72,
            bounty_ttl: 108,
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_artifact_posting_profile(agent, profile.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::artifact_posting_profile(&view, agent),
            Some(profile.clone())
        );
        assert_eq!(
            GoalBeliefView::artifact_posting_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn exploration_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = ExplorationProfile {
            curiosity_weight: Permille::new(275).unwrap(),
            need_activation_threshold: Permille::new(350).unwrap(),
            frontier_depth: 4,
            acquisition_failure_threshold: 6,
            exploration_arrival_boost: Permille::new(650).unwrap(),
            max_consecutive_explorations: 5,
            visit_lookback_ticks: 17,
            negative_survey_damping_window: 123,
            negative_survey_damping_strength: Permille::new(625).unwrap(),
            consecutive_exploration_count: 1,
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_exploration_profile(agent, profile)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::exploration_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::exploration_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn exploration_profile_returns_default_for_live_agent() {
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
            ProfileBeliefView::exploration_profile(&view, agent),
            Some(ExplorationProfile::default())
        );
        assert_eq!(
            GoalBeliefView::exploration_profile(&view, agent),
            Some(ExplorationProfile::default())
        );
    }

    #[test]
    fn acquisition_exhaustion_count_returns_actor_tracker_count_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let mut tracker = worldwake_core::AcquisitionExhaustionTracker::default();
            tracker.increment(HomeostaticNeedId::Hunger);
            tracker.increment(HomeostaticNeedId::Hunger);
            tracker.increment(HomeostaticNeedId::Thirst);
            txn.set_component_acquisition_exhaustion_tracker(agent, tracker)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::acquisition_exhaustion_count(
                &view,
                agent,
                HomeostaticNeedId::Hunger
            ),
            2
        );
        assert_eq!(
            GoalBeliefView::acquisition_exhaustion_count(&view, agent, HomeostaticNeedId::Hunger),
            2
        );
        assert_eq!(
            GoalBeliefView::acquisition_exhaustion_count(&view, agent, HomeostaticNeedId::Thirst),
            1
        );
    }

    #[test]
    fn acquisition_exhaustion_count_returns_zero_for_non_self_and_default_live_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            let mut tracker = worldwake_core::AcquisitionExhaustionTracker::default();
            tracker.increment(HomeostaticNeedId::Hunger);
            txn.set_component_acquisition_exhaustion_tracker(other, tracker)
                .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::acquisition_exhaustion_count(
                &view,
                agent,
                HomeostaticNeedId::Hunger
            ),
            0
        );
        assert_eq!(
            GoalBeliefView::acquisition_exhaustion_count(&view, agent, HomeostaticNeedId::Hunger),
            0
        );
        assert_eq!(
            ProfileBeliefView::acquisition_exhaustion_count(
                &view,
                other,
                HomeostaticNeedId::Hunger
            ),
            0
        );
        assert_eq!(
            GoalBeliefView::acquisition_exhaustion_count(&view, other, HomeostaticNeedId::Hunger),
            0
        );
    }

    #[test]
    fn obligation_satiation_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = ObligationSatiationProfile {
            satiation_threshold: 5,
            ..ObligationSatiationProfile::default()
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_obligation_satiation_profile(agent, profile.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::obligation_satiation_profile(&view, agent),
            profile
        );
        assert_eq!(
            GoalBeliefView::obligation_satiation_profile(&view, agent),
            profile
        );
    }

    #[test]
    fn obligation_execution_tracker_returns_default_when_absent() {
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
            ProfileBeliefView::obligation_execution_tracker(&view, agent),
            ObligationExecutionTracker::default()
        );
        assert_eq!(
            GoalBeliefView::obligation_execution_tracker(&view, agent),
            ObligationExecutionTracker::default()
        );
    }

    #[test]
    fn obligation_execution_tracker_returns_actor_tracker_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let tracker = ObligationExecutionTracker {
            completion_ticks: vec![Tick(3), Tick(8)],
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_obligation_execution_tracker(agent, tracker.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::obligation_execution_tracker(&view, agent),
            tracker
        );
        assert_eq!(
            GoalBeliefView::obligation_execution_tracker(&view, agent),
            tracker
        );
    }

    #[test]
    fn cognitive_profile_returns_actor_profile_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let profile = CognitiveProfile {
            max_plan_depth: 12,
            landmark_extraction_depth: 6,
            ..CognitiveProfile::default()
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_cognitive_profile(agent, profile).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::cognitive_profile(&view, agent),
            Some(profile)
        );
        assert_eq!(
            GoalBeliefView::cognitive_profile(&view, agent),
            Some(profile)
        );
    }

    #[test]
    fn cognitive_profile_returns_none_for_non_self_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(ProfileBeliefView::cognitive_profile(&view, other), None);
        assert_eq!(GoalBeliefView::cognitive_profile(&view, other), None);
    }

    #[test]
    fn opportunity_profiles_return_actor_profiles_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let risk_weight = RiskWeightProfile {
            theft_aversion: Permille::new_unchecked(300),
            exposure_aversion: Permille::new_unchecked(450),
            threat_aversion: Permille::new_unchecked(700),
        };
        let law_abiding = LawAbidingProfile {
            criminal_threshold: Permille::new_unchecked(650),
            social_norm_weight: Permille::new_unchecked(250),
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_risk_weight_profile(agent, risk_weight)
                .unwrap();
            txn.set_component_law_abiding_profile(agent, law_abiding)
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            ProfileBeliefView::risk_weight_profile(&view, agent),
            Some(risk_weight)
        );
        assert_eq!(
            GoalBeliefView::risk_weight_profile(&view, agent),
            Some(risk_weight)
        );
        assert_eq!(
            ProfileBeliefView::law_abiding_profile(&view, agent),
            Some(law_abiding)
        );
        assert_eq!(
            GoalBeliefView::law_abiding_profile(&view, agent),
            Some(law_abiding)
        );
    }

    #[test]
    fn memory_accessors_return_actor_memories_when_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let discrepancy = sample_discrepancy_memory();
        let blocker = sample_blocker_memory();
        let repair = sample_repair_memory();
        let learned = sample_learned_opportunity_memory();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_discrepancy_memory(agent, discrepancy.clone())
                .unwrap();
            txn.set_component_blocker_memory(agent, blocker.clone())
                .unwrap();
            txn.set_component_repair_memory(agent, repair.clone())
                .unwrap();
            txn.set_component_learned_opportunity_memory(agent, learned.clone())
                .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            GoalBeliefView::discrepancy_memory(&view, agent),
            Some(&discrepancy)
        );
        assert_eq!(GoalBeliefView::blocker_memory(&view, agent), Some(&blocker));
        assert_eq!(GoalBeliefView::repair_memory(&view, agent), Some(&repair));
        assert_eq!(
            GoalBeliefView::learned_opportunity_memory(&view, agent),
            Some(&learned)
        );
    }

    #[test]
    fn memory_accessors_return_none_for_non_self_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            txn.set_component_discrepancy_memory(other, sample_discrepancy_memory())
                .unwrap();
            txn.set_component_blocker_memory(other, sample_blocker_memory())
                .unwrap();
            txn.set_component_repair_memory(other, sample_repair_memory())
                .unwrap();
            txn.set_component_learned_opportunity_memory(
                other,
                sample_learned_opportunity_memory(),
            )
            .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(GoalBeliefView::discrepancy_memory(&view, other), None);
        assert_eq!(GoalBeliefView::blocker_memory(&view, other), None);
        assert_eq!(GoalBeliefView::repair_memory(&view, other), None);
        assert_eq!(
            GoalBeliefView::learned_opportunity_memory(&view, other),
            None
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
            ProfileBeliefView::preference_profile(&view, agent),
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
            crate::SocialBeliefView::belief_confidence_policy(&view, agent),
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

        let _ = crate::SocialBeliefView::belief_confidence_policy(&view, other);
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
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, workstation)
        };

        let empty_beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &empty_beliefs);
        assert!(
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, place)
                .iter()
                .any(|(adjacent, _)| *adjacent == remote_place),
            "public route topology should remain available"
        );
        assert_eq!(
            EntityBeliefView::entity_kind(&view, remote_place),
            Some(EntityKind::Place),
            "public route knowledge should include place identity"
        );
        assert!(
            FacilityBeliefView::matching_workstations_at(
                &view,
                remote_place,
                WorkstationTag::OrchardRow
            )
            .is_empty(),
            "remote workstation discovery must not come from authoritative scans"
        );
        assert!(
            FacilityBeliefView::resource_sources_at(&view, remote_place, CommodityKind::Apple)
                .is_empty(),
            "remote resource-source discovery must not come from authoritative scans"
        );
        assert_eq!(
            FacilityBeliefView::workstation_tag(&view, workstation),
            None
        );
        assert_eq!(
            FacilityBeliefView::resource_source(&view, workstation),
            None
        );

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
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            FacilityBeliefView::matching_workstations_at(
                &view,
                remote_place,
                WorkstationTag::OrchardRow
            ),
            vec![workstation]
        );
        assert_eq!(
            FacilityBeliefView::resource_sources_at(&view, remote_place, CommodityKind::Apple),
            vec![workstation]
        );
        assert_eq!(
            FacilityBeliefView::workstation_tag(&view, workstation),
            Some(WorkstationTag::OrchardRow)
        );
        assert_eq!(
            FacilityBeliefView::resource_source(&view, workstation),
            Some(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(9),
                max_quantity: Quantity(12),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            }),
            "belief-side facility/resource knowledge should remain stale until refreshed"
        );
    }

    #[test]
    fn remote_facility_controller_change_without_carrier_stays_hidden() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let observer_place = places[0];
        let market = places[1];
        let (observer, believed_staff, hidden_controller, facility) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let believed_staff = txn.create_agent("Staff", ControlSource::Ai).unwrap();
            let hidden_controller = txn
                .create_agent("Hidden Controller", ControlSource::Ai)
                .unwrap();
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(observer, observer_place).unwrap();
            txn.set_ground_location(believed_staff, market).unwrap();
            txn.set_ground_location(hidden_controller, market).unwrap();
            txn.set_ground_location(facility, market).unwrap();
            commit_txn(txn);
            (observer, believed_staff, hidden_controller, facility)
        };

        let mut beliefs = AgentBeliefStore::new();
        let mut staff_belief = entity_belief(market, true, 0, 1);
        staff_belief.believed_kind = Some(EntityKind::Agent);
        beliefs.update_entity(believed_staff, staff_belief);
        let mut facility_belief = entity_belief(market, true, 0, 1);
        facility_belief.believed_kind = Some(EntityKind::Facility);
        beliefs.update_entity(facility, facility_belief);

        let view = PerAgentBeliefView::new(observer, &world, &beliefs);
        assert_eq!(view.facility_controller_at(facility, market), None);

        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_owner(facility, hidden_controller).unwrap();
            commit_txn(txn);
        }
        assert!(
            world
                .can_exercise_control(hidden_controller, facility)
                .is_ok()
        );

        let view = PerAgentBeliefView::new(observer, &world, &beliefs);
        assert_eq!(
            view.facility_controller_at(facility, market),
            None,
            "hidden authoritative control must not surface as a remote seller/controller"
        );
        assert!(
            !SpatialBeliefView::entities_at(&view, market).contains(&hidden_controller),
            "the hidden controller must remain outside the observer's believed-present candidates"
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, origin),
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, origin),
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, origin),
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, origin),
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
            SpatialBeliefView::adjacent_places_with_travel_ticks(&view, origin),
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
            CombatBeliefView::current_attackers_of(&view, agent),
            vec![attacker]
        );
        assert_eq!(
            TemporalBeliefView::estimate_duration(
                &view,
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
            CombatBeliefView::visible_hostiles_for(&view, agent).is_empty(),
            "dead believed hostiles should not continue to project danger"
        );
        assert!(
            CombatBeliefView::hostile_targets_of(&view, agent).is_empty(),
            "dead believed hostiles should not remain actionable hostile targets"
        );
    }

    #[test]
    fn remote_believed_hostiles_are_actionable_but_not_locally_visible() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let remote = places[1];
        let (agent, attacker) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let attacker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(attacker, remote).unwrap();
            txn.add_hostility(agent, attacker).unwrap();
            commit_txn(txn);
            (agent, attacker)
        };

        let mut beliefs = AgentBeliefStore::new();
        let mut attacker_belief = entity_belief(remote, true, 0, 1);
        attacker_belief.believed_kind = Some(EntityKind::Agent);
        beliefs.update_entity(attacker, attacker_belief);
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            CombatBeliefView::hostile_targets_of(&view, agent),
            vec![attacker],
            "remote pursuit needs known hostile targets to remain actionable"
        );
        assert!(
            CombatBeliefView::visible_hostiles_for(&view, agent).is_empty(),
            "remote hostile targets should not project same-place visible danger"
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
            TemporalBeliefView::estimate_duration(
                &view,
                agent,
                &DurationExpr::ActorDefendStance,
                &[],
                &ActionPayload::None,
            ),
            Some(ActionDuration::new(10))
        );
    }

    #[test]
    fn estimate_duration_hides_consult_record_timing_without_believed_record_snapshot() {
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
            TemporalBeliefView::estimate_duration(
                &view,
                agent,
                &DurationExpr::ConsultRecord { target_index: 0 },
                &[record],
                &ActionPayload::None,
            ),
            None
        );
    }

    #[test]
    fn estimate_duration_uses_believed_record_snapshot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, record, record_data) = {
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
            let record_data = worldwake_core::RecordData {
                record_kind: worldwake_core::RecordKind::OfficeRegister,
                home_place: place,
                issuer: agent,
                consultation_ticks: 8,
                max_entries_per_consult: 4,
                entries: Vec::new(),
                next_entry_id: 0,
            };
            let record = txn.create_record(record_data.clone()).unwrap();
            commit_txn(txn);
            (agent, record, record_data)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_believed_record_data(
            record,
            worldwake_core::BelievedRecordDataSnapshot {
                data: record_data.clone(),
                source: worldwake_core::InstitutionalSnapshotSource::RecordConsultation { record },
                learned_tick: Tick(2),
                learned_at: Some(place),
            },
        );
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            crate::PoliticalBeliefView::record_data(&view, record),
            Some(record_data)
        );
        assert_eq!(
            TemporalBeliefView::estimate_duration(
                &view,
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

        assert_eq!(crate::PoliticalBeliefView::office_data(&view, office), None);
        assert_eq!(
            match BelievedAuthorityView::believed_office_holder(&view, office) {
                BeliefRead::Known(value) | BeliefRead::Stale(value) => Some(value.value),
                BeliefRead::Unknown => None,
            },
            Some(Some(holder))
        );
        assert_eq!(
            crate::PoliticalBeliefView::believed_membership(&view, faction, agent),
            InstitutionalBeliefRead::Certain(true)
        );
        assert_eq!(
            crate::PoliticalBeliefView::loyalty_to(&view, agent, holder),
            Some(Permille::new(620).unwrap())
        );
        assert_eq!(
            crate::PoliticalBeliefView::believed_support_declaration(&view, office, agent),
            InstitutionalBeliefRead::Certain(Some(holder))
        );
    }

    #[test]
    fn record_and_office_data_do_not_read_live_truth_for_believed_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, record, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Steward".to_string(),
                    seat: place,
                    jurisdiction: BTreeSet::from([place]),
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 6,
                    vacancy_since: Some(Tick(2)),
                },
            )
            .unwrap();
            let record = txn
                .create_record(worldwake_core::RecordData {
                    record_kind: RecordKind::OfficeRegister,
                    home_place: place,
                    issuer: office,
                    consultation_ticks: 4,
                    max_entries_per_consult: 6,
                    entries: Vec::new(),
                    next_entry_id: 0,
                })
                .unwrap();
            commit_txn(txn);
            (agent, record, office)
        };

        assert!(world.get_component_record_data(record).is_some());
        assert!(world.get_component_office_data(office).is_some());

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(record, entity_belief(place, true, 0, 2));
        beliefs.update_entity(office, entity_belief(place, true, 0, 2));
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(2),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(3),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(crate::PoliticalBeliefView::record_data(&view, record), None);
        assert_eq!(crate::PoliticalBeliefView::office_data(&view, office), None);
        assert!(matches!(
            BelievedAuthorityView::believed_office_holder(&view, office),
            BeliefRead::Known(holder) | BeliefRead::Stale(holder) if holder.value.is_none()
        ));
    }

    #[test]
    fn record_and_office_data_read_believed_snapshots_without_live_fallback() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, record, office, record_data, office_data) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            let office_data = OfficeData {
                title: "Steward".to_string(),
                seat: place,
                jurisdiction: BTreeSet::from([place]),
                succession_law: SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 6,
                vacancy_since: Some(Tick(2)),
            };
            txn.set_component_office_data(office, office_data.clone())
                .unwrap();
            let record_data = worldwake_core::RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: place,
                issuer: office,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            };
            let record = txn.create_record(record_data.clone()).unwrap();
            commit_txn(txn);
            (agent, record, office, record_data, office_data)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_believed_record_data(
            record,
            worldwake_core::BelievedRecordDataSnapshot {
                data: record_data.clone(),
                source: worldwake_core::InstitutionalSnapshotSource::RecordConsultation { record },
                learned_tick: Tick(3),
                learned_at: Some(place),
            },
        );
        beliefs.record_believed_office_data(
            office,
            worldwake_core::BelievedOfficeDataSnapshot {
                data: office_data.clone(),
                source: worldwake_core::InstitutionalSnapshotSource::RecordConsultation { record },
                learned_tick: Tick(3),
                learned_at: Some(place),
            },
        );
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            crate::PoliticalBeliefView::record_data(&view, record),
            Some(record_data)
        );
        assert_eq!(
            crate::PoliticalBeliefView::office_data(&view, office),
            Some(office_data)
        );
    }

    #[test]
    fn loyalty_to_requires_explicit_target_belief_not_colocation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, target) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_loyalty(agent, target, Permille::new(620).unwrap())
                .unwrap();
            commit_txn(txn);
            (agent, target)
        };

        let empty_beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &empty_beliefs);
        assert_eq!(
            crate::PoliticalBeliefView::loyalty_to(&view, agent, target),
            None
        );

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(target, entity_belief(place, true, 0, 2));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            crate::PoliticalBeliefView::loyalty_to(&view, agent, target),
            Some(Permille::new(620).unwrap())
        );
    }

    #[test]
    fn stock_storage_policy_requires_explicit_facility_belief_not_colocation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, facility, policy) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let (facility, stock_container, display_container) = txn
                .create_merchant_facility(place, agent, LoadUnits(10), Some(LoadUnits(3)))
                .unwrap();
            let policy = StockStoragePolicy {
                stock_container,
                display_container,
            };
            commit_txn(txn);
            (agent, facility, policy)
        };

        assert_eq!(
            world.get_component_stock_storage_policy(facility),
            Some(&policy)
        );

        let empty_beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &empty_beliefs);
        assert_eq!(
            crate::FacilityBeliefView::stock_storage_policy(&view, facility),
            None
        );

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(facility, entity_belief(place, true, 0, 2));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            crate::FacilityBeliefView::stock_storage_policy(&view, facility),
            Some(policy)
        );
    }

    #[test]
    fn believed_authority_office_holder_reads_from_institutional_belief_store() {
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
            match BelievedAuthorityView::believed_office_holder(&view, office) {
                BeliefRead::Known(value) | BeliefRead::Stale(value) => Some(value.value),
                BeliefRead::Unknown => None,
            },
            Some(Some(holder))
        );
    }

    #[test]
    fn believed_authority_office_holder_keeps_local_office_unknown_without_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, _holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.set_ground_location(office, place).unwrap();
            txn.assign_office(office, holder).unwrap();
            commit_txn(txn);
            (agent, holder, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(matches!(
            BelievedAuthorityView::believed_office_holder(&view, office),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn believed_authority_office_holder_keeps_remote_office_unknown_without_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (agent_place, office_place) = {
            let mut places = world.topology().place_ids();
            (places.next().unwrap(), places.next().unwrap())
        };
        let (agent, _holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_ground_location(agent, agent_place).unwrap();
            txn.set_ground_location(holder, office_place).unwrap();
            txn.set_ground_location(office, office_place).unwrap();
            txn.assign_office(office, holder).unwrap();
            commit_txn(txn);
            (agent, holder, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(matches!(
            BelievedAuthorityView::believed_office_holder(&view, office),
            BeliefRead::Unknown
        ));
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
            crate::PoliticalBeliefView::believed_force_controller(&view, office),
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
            crate::PoliticalBeliefView::believed_support_declaration(&view, office, supporter),
            InstitutionalBeliefRead::Certain(Some(candidate))
        );
        assert_eq!(
            crate::PoliticalBeliefView::believed_support_declarations_for_office(&view, office),
            vec![(supporter, InstitutionalBeliefRead::Certain(Some(candidate)),)]
        );
    }

    #[test]
    fn believed_owner_of_ignores_authoritative_owner_without_explicit_claim() {
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
        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn believed_owner_of_returns_unknown_for_unowned_entity_without_explicit_claim() {
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
        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn believed_owner_of_returns_unknown_for_unknown_entity() {
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
        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn believed_authority_view_on_per_agent_view_starts_unknown() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, Some(agent))
                .unwrap();
            let office = txn.create_entity(EntityKind::Office);
            txn.set_ground_location(office, place).unwrap();
            commit_txn(txn);
            (agent, lot, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            BelievedAuthorityView::believed_holder_of(&view, lot),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            BelievedAuthorityView::believed_access_right(&view, agent, lot),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            BelievedAuthorityView::believed_jurisdiction(&view, place),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            BelievedAuthorityView::believed_office_holder(&view, office),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn believed_authority_view_reads_explicit_owner_and_holder_claims() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, owner, holder, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let owner = txn.create_agent("Owner", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(owner, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, Some(owner))
                .unwrap();
            txn.set_possessor(lot, holder).unwrap();
            commit_txn(txn);
            (agent, owner, holder, lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_entity_claim(sample_claim(
            1,
            lot,
            EntityBeliefAspect::Owner,
            ClaimValue::Entity(Some(owner)),
            4,
            900,
        ));
        beliefs.record_entity_claim(sample_claim(
            2,
            lot,
            EntityBeliefAspect::Holder,
            ClaimValue::Entity(Some(holder)),
            5,
            900,
        ));

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(6), &world, &beliefs);

        match BelievedAuthorityView::believed_owner_of(&view, lot) {
            BeliefRead::Known(value) => assert_eq!(value.value, owner),
            other => panic!("expected known owner belief, got {other:?}"),
        }
        match BelievedAuthorityView::believed_holder_of(&view, lot) {
            BeliefRead::Known(value) => assert_eq!(value.value, holder),
            other => panic!("expected known holder belief, got {other:?}"),
        }
    }

    #[test]
    fn believed_authority_view_collapses_absent_and_conflicting_claims_to_unknown() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, first_owner, second_owner, lot, empty_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let first_owner = txn.create_agent("Owner One", ControlSource::Ai).unwrap();
            let second_owner = txn.create_agent("Owner Two", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(first_owner, place).unwrap();
            txn.set_ground_location(second_owner, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(
                    CommodityKind::Bread,
                    Quantity(3),
                    place,
                    Some(first_owner),
                )
                .unwrap();
            let empty_lot = txn
                .create_item_lot_with_owner(CommodityKind::Water, Quantity(1), place, None)
                .unwrap();
            commit_txn(txn);
            (agent, first_owner, second_owner, lot, empty_lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_entity_claim(sample_claim(
            1,
            lot,
            EntityBeliefAspect::Owner,
            ClaimValue::Entity(Some(first_owner)),
            4,
            900,
        ));
        beliefs.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(2),
            subject: lot,
            aspect: EntityBeliefAspect::Owner,
            value: ClaimValue::Entity(Some(second_owner)),
            source: PerceptionSource::Report {
                from: second_owner,
                chain_len: 1,
            },
            acquired_tick: Tick(5),
            claimed_event_tick: Some(Tick(5)),
            confidence: Permille::new(900).unwrap(),
            refuted_at_tick: None,
        });
        beliefs.record_entity_claim(sample_claim(
            3,
            empty_lot,
            EntityBeliefAspect::Owner,
            ClaimValue::Entity(None),
            5,
            900,
        ));

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(6), &world, &beliefs);

        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, empty_lot),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn local_physical_observation_view_colocated_entities_matches_legacy_read() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let local_place = places[0];
        let remote_place = places[1];
        let (agent, local_lot, remote_agent) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, local_place).unwrap();
            let local_lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), local_place, None)
                .unwrap();
            let remote_agent = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(remote_agent, remote_place).unwrap();
            commit_txn(txn);
            (agent, local_lot, remote_agent)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new_at_tick(agent, Tick(9), &world, &beliefs);
        let observed = LocalPhysicalObservationView::colocated_entities(&view, agent);

        assert!(observed.value.contains(&agent));
        assert!(observed.value.contains(&local_lot));
        assert!(!observed.value.contains(&remote_agent));
        let mut expected = world.entities_effectively_at(local_place);
        expected.sort();
        expected.dedup();
        assert_eq!(observed.value, expected);
        assert_eq!(observed.observed_tick, Tick(9));
        assert_eq!(observed.source, ObservationSource::CoLocatedSameTick);
    }

    #[test]
    fn local_physical_observation_view_gates_subject_reads_by_colocation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let local_place = places[0];
        let remote_place = places[1];
        let (agent, local_lot, remote_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, local_place).unwrap();
            let local_lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), local_place, None)
                .unwrap();
            let remote_lot = txn
                .create_item_lot_with_owner(CommodityKind::Water, Quantity(5), remote_place, None)
                .unwrap();
            commit_txn(txn);
            (agent, local_lot, remote_lot)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new_at_tick(agent, Tick(12), &world, &beliefs);

        let local_quantity =
            LocalPhysicalObservationView::observed_item_lot_quantity(&view, local_lot);
        let remote_quantity =
            LocalPhysicalObservationView::observed_item_lot_quantity(&view, remote_lot);
        let local_kind = LocalPhysicalObservationView::observed_entity_kind(&view, local_lot);
        let remote_kind = LocalPhysicalObservationView::observed_entity_kind(&view, remote_lot);

        assert_eq!(local_quantity.value, Some(Quantity(3)));
        assert_eq!(local_quantity.observed_tick, Tick(12));
        assert_eq!(remote_quantity.value, None);
        assert_eq!(local_kind.value, Some(EntityKind::ItemLot));
        assert_eq!(remote_kind.value, None);
    }

    #[test]
    fn debug_world_view_reports_authoritative_entity_state() {
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

        let world_ref = &world;
        assert_eq!(
            DebugWorldView::world_entity_state(&world_ref, lot),
            EntityState {
                kind: Some(EntityKind::ItemLot),
                place: Some(place),
                alive: true,
                container: None,
                possessor: None,
            }
        );
        assert_eq!(DebugWorldView::world_owner_of(&world_ref, lot), Some(agent));
        assert_eq!(
            DebugWorldView::world_location_of(&world_ref, lot),
            Some(place)
        );
        assert!(DebugWorldView::world_inventory_of(&world_ref, agent).is_empty());
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
            InventoryBeliefView::locally_observed_commodity_quantity(
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
    fn believed_owner_of_does_not_use_self_ownership_as_belief() {
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

        // Agent has no explicit authority belief claim; live ownership is not a belief.
        let beliefs = AgentBeliefStore::new();

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert!(matches!(
            BelievedAuthorityView::believed_owner_of(&view, lot),
            BeliefRead::Unknown
        ));
    }

    #[test]
    fn entity_kind_returns_item_lot_for_self_possessed_unknown_item() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_possessor(lot, agent).unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            EntityBeliefView::entity_kind(&view, lot),
            Some(EntityKind::ItemLot)
        );
        assert_eq!(
            InventoryBeliefView::item_lot_commodity(&view, lot),
            Some(CommodityKind::Water)
        );
    }

    #[test]
    fn co_located_unknown_item_lot_exposes_physical_inventory_facts() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, loose_lot, held_lot, holder) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            let loose_lot = txn
                .create_item_lot_with_owner(CommodityKind::Water, Quantity(2), place, None)
                .unwrap();
            let held_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(held_lot, place).unwrap();
            txn.set_possessor(held_lot, holder).unwrap();
            commit_txn(txn);
            (agent, loose_lot, held_lot, holder)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            InventoryBeliefView::item_lot_commodity(&view, loose_lot),
            Some(CommodityKind::Water)
        );
        assert_eq!(
            InventoryBeliefView::direct_container(&view, loose_lot),
            None
        );
        assert_eq!(
            InventoryBeliefView::direct_possessor(&view, loose_lot),
            None
        );
        assert_eq!(
            InventoryBeliefView::item_lot_commodity(&view, held_lot),
            Some(CommodityKind::Bread)
        );
        assert_eq!(
            InventoryBeliefView::direct_possessor(&view, held_lot),
            Some(holder)
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
            ControlBeliefView::believed_rights(&view, agent, lot),
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

        assert!(ControlBeliefView::believed_rights(&view, other, lot).is_empty());
    }

    #[test]
    fn can_control_returns_false_for_belief_inaccessible_authoritatively_controlled_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let seat = places[0];
        let remote_place = places[1];
        let (agent, item) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, seat).unwrap();
            let office = txn.create_office("Storehouse Steward").unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Steward".to_string(),
                    seat,
                    jurisdiction: BTreeSet::from([seat, remote_place]),
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 8,
                    vacancy_since: None,
                },
            )
            .unwrap();
            create_record(&mut txn, seat, agent, RecordKind::OfficeRegister);
            txn.assign_office(office, agent).unwrap();
            let item = txn
                .create_item_lot_with_owner(
                    CommodityKind::Bread,
                    Quantity(1),
                    remote_place,
                    Some(office),
                )
                .unwrap();
            commit_txn(txn);
            (agent, item)
        };

        assert!(world.can_exercise_control(agent, item).is_ok());

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(!ControlBeliefView::can_control(&view, agent, item));
    }

    #[test]
    fn can_control_returns_true_for_belief_accessible_controlled_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let seat = places[0];
        let remote_place = places[1];
        let (agent, item) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, seat).unwrap();
            let item = txn
                .create_item_lot_with_owner(
                    CommodityKind::Bread,
                    Quantity(1),
                    remote_place,
                    Some(agent),
                )
                .unwrap();
            commit_txn(txn);
            (agent, item)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(item, entity_belief(remote_place, true, 1, 10));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(ControlBeliefView::can_control(&view, agent, item));
    }

    #[test]
    fn can_control_returns_false_for_colocated_unowned_item_without_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(1), place, None)
                .unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(!ControlBeliefView::can_control(&view, agent, lot));
    }

    #[test]
    fn has_control_returns_false_for_non_self_live_control_source_without_belief() {
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
        beliefs.update_entity(other, entity_belief(place, true, 0, 2));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(ControlBeliefView::has_control(&view, agent));
        assert!(!ControlBeliefView::has_control(&view, other));
    }

    #[test]
    fn remote_control_and_rights_changes_without_carrier_stay_invisible() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let local_place = places[0];
        let remote_place = places[1];
        let (agent, other, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, local_place).unwrap();
            txn.set_ground_location(other, remote_place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(1), remote_place, None)
                .unwrap();
            commit_txn(txn);
            (agent, other, lot)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert!(!ControlBeliefView::has_control(&view, other));
        assert!(!ControlBeliefView::can_control(&view, agent, lot));
        assert!(ControlBeliefView::believed_rights(&view, agent, lot).is_empty());

        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_owner(lot, agent).unwrap();
            commit_txn(txn);
        }

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert!(!ControlBeliefView::has_control(&view, other));
        assert!(!ControlBeliefView::can_control(&view, agent, lot));
        assert!(ControlBeliefView::believed_rights(&view, agent, lot).is_empty());
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
        assert!(!ControlBeliefView::can_control(&view, holder, item));
        assert_eq!(
            ControlBeliefView::believed_rights(&view, holder, item),
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
            CombatBeliefView::courage(&view, agent),
            Some(Permille::new(750).unwrap())
        );
        // Other agent: returns believed courage
        assert_eq!(
            CombatBeliefView::courage(&view, other),
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

        assert_eq!(CombatBeliefView::courage(&view, other), None);
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

        assert_eq!(CombatBeliefView::courage(&view, unknown), None);
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

        assert_eq!(CombatBeliefView::courage(&view, agent), None);
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
            crate::PoliticalBeliefView::believed_membership(&view, faction, agent),
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
            crate::PoliticalBeliefView::believed_faction_rally_point(&view, faction),
            InstitutionalBeliefRead::Certain(Some(rally_place))
        );
    }

    #[test]
    fn bandit_policy_entity_methods_are_gated_to_own_bandit_factions() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, faction, other_faction) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let faction = txn.create_faction("River Pact").unwrap();
            let other_faction = txn.create_faction("Hill Knives").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.add_member(agent, faction).unwrap();
            txn.set_component_bandit_faction_policy(
                faction,
                BanditFactionPolicy {
                    rally_place: Some(place),
                    establishment_duration_ticks: NonZeroU32::new(8).unwrap(),
                    min_regroup_count: 1,
                    abandonment_grace_ticks: NonZeroU32::new(4).unwrap(),
                    flee_wound_threshold: Permille::new(650).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_bandit_faction_policy(
                other_faction,
                BanditFactionPolicy {
                    rally_place: Some(place),
                    establishment_duration_ticks: NonZeroU32::new(12).unwrap(),
                    min_regroup_count: 1,
                    abandonment_grace_ticks: NonZeroU32::new(5).unwrap(),
                    flee_wound_threshold: Permille::new(800).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, faction, other_faction)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            EntityBeliefView::bandit_flee_wound_threshold(&view, faction),
            Some(Permille::new(650).unwrap())
        );
        assert_eq!(
            EntityBeliefView::bandit_camp_establishment_ticks(&view, faction),
            Some(NonZeroU32::new(8).unwrap())
        );
        assert_eq!(
            EntityBeliefView::bandit_flee_wound_threshold(&view, other_faction),
            None
        );
        assert_eq!(
            EntityBeliefView::bandit_camp_establishment_ticks(&view, other_faction),
            None
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

    #[test]
    fn believed_target_location_marks_disputed_when_multiple_places_are_claimed() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (home, other_place) = {
            let mut places = world.topology().place_ids();
            (places.next().unwrap(), places.next().unwrap())
        };
        let (agent, target) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, home).unwrap();
            txn.set_ground_location(target, other_place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    claim_confidence_threshold: Permille::new(300).unwrap(),
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, target)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_entity_claim(sample_claim(
            1,
            target,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(home)),
            7,
            950,
        ));
        beliefs.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(2),
            subject: target,
            aspect: EntityBeliefAspect::Location,
            value: ClaimValue::Place(Some(other_place)),
            source: PerceptionSource::Report {
                from: entity(77),
                chain_len: 1,
            },
            acquired_tick: Tick(9),
            claimed_event_tick: Some(Tick(9)),
            confidence: Permille::new(975).unwrap(),
            refuted_at_tick: None,
        });

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(10), &world, &beliefs);
        let value = EntityBeliefView::believed_target_location(&view, agent, target);

        assert_eq!(value.value, Some(other_place));
        assert_eq!(value.status, crate::belief_view::BeliefStatus::Disputed);
        assert_eq!(value.claimed_event_tick, Some(Tick(9)));
    }

    #[test]
    fn believed_target_location_surfaces_contradicted_when_only_refuted_claim_remains() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (home, other_place) = {
            let mut places = world.topology().place_ids();
            (places.next().unwrap(), places.next().unwrap())
        };
        let (agent, target) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, home).unwrap();
            txn.set_ground_location(target, other_place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    claim_confidence_threshold: Permille::new(300).unwrap(),
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, target)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_entity_claim(EntityBeliefClaim {
            refuted_at_tick: Some(Tick(9)),
            ..sample_claim(
                1,
                target,
                EntityBeliefAspect::Location,
                ClaimValue::Place(Some(other_place)),
                7,
                950,
            )
        });

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(10), &world, &beliefs);
        let value = EntityBeliefView::believed_target_location(&view, agent, target);

        assert_eq!(value.value, Some(other_place));
        assert_eq!(value.status, crate::belief_view::BeliefStatus::Contradicted);
        assert_eq!(value.claimed_event_tick, Some(Tick(7)));
    }

    #[test]
    fn believed_entities_at_returns_claimed_entities_with_metadata() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    claim_confidence_threshold: Permille::new(300).unwrap(),
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            agent
        };
        let believed_other = entity(90);

        let mut beliefs = AgentBeliefStore::new();
        let mut state = entity_belief(place, true, 0, 9);
        state.believed_kind = Some(EntityKind::Agent);
        beliefs.update_entity(believed_other, state);
        beliefs.record_entity_claim(sample_claim(
            1,
            believed_other,
            EntityBeliefAspect::Location,
            ClaimValue::Place(Some(place)),
            9,
            980,
        ));

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(10), &world, &beliefs);
        let entities =
            SpatialBeliefView::believed_entities_at(&view, agent, place, EntityKind::Agent);

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].value, believed_other);
        assert_eq!(
            entities[0].status,
            crate::belief_view::BeliefStatus::Certain
        );
        assert_eq!(entities[0].claimed_event_tick, Some(Tick(9)));
    }

    #[test]
    fn believed_commodity_stock_projects_place_inventory_claims() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    claim_confidence_threshold: Permille::new(300).unwrap(),
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            agent
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.record_entity_claim(sample_claim(
            1,
            place,
            EntityBeliefAspect::Inventory(CommodityKind::Bread),
            ClaimValue::Quantity(Quantity(6)),
            9,
            920,
        ));

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(10), &world, &beliefs);
        let stock = InventoryBeliefView::believed_commodity_stock(
            &view,
            agent,
            place,
            CommodityKind::Bread,
        );

        assert_eq!(stock.value, Quantity(6));
        assert_eq!(stock.status, crate::belief_view::BeliefStatus::Certain);
        assert_eq!(stock.claimed_event_tick, Some(Tick(9)));
    }

    #[test]
    fn belief_view_last_harvest_trace_co_located_only() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let near_place = places[0];
        let remote_place = world.topology().neighbors(near_place)[0];

        let trace = LastHarvestTrace {
            entries: vec![
                HarvestTraceEntry {
                    harvester: EntityId {
                        slot: 7,
                        generation: 0,
                    },
                    tick: Tick(11),
                    quantity: 3,
                    partial: false,
                },
                HarvestTraceEntry {
                    harvester: EntityId {
                        slot: 9,
                        generation: 0,
                    },
                    tick: Tick(15),
                    quantity: 1,
                    partial: true,
                },
            ],
        };

        let (agent, near_source, remote_source) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let near_source = txn.create_entity(EntityKind::Facility);
            let remote_source = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(agent, near_place).unwrap();
            txn.set_ground_location(near_source, near_place).unwrap();
            txn.set_ground_location(remote_source, remote_place)
                .unwrap();
            txn.set_component_workstation_marker(
                near_source,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_workstation_marker(
                remote_source,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                near_source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(8),
                    max_quantity: Quantity(12),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_resource_source(
                remote_source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(5),
                    max_quantity: Quantity(12),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_last_harvest_trace(near_source, trace.clone())
                .unwrap();
            txn.set_component_last_harvest_trace(remote_source, trace.clone())
                .unwrap();
            commit_txn(txn);
            (agent, near_source, remote_source)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        // Co-located: authoritative trace is visible.
        assert_eq!(
            FacilityBeliefView::last_harvest_trace(&view, near_source),
            Some(trace.clone())
        );
        assert_eq!(
            GoalBeliefView::last_harvest_trace(&view, near_source),
            Some(trace)
        );

        // Remote: trace is gated off, even though the agent could in
        // principle hold a believed-entity snapshot for the source.
        assert_eq!(
            FacilityBeliefView::last_harvest_trace(&view, remote_source),
            None
        );
        assert_eq!(
            GoalBeliefView::last_harvest_trace(&view, remote_source),
            None
        );
    }

    #[test]
    fn belief_view_resource_extraction_queues_co_located_only() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let near_place = places[0];
        let remote_place = world.topology().neighbors(near_place)[0];

        let queues = ResourceExtractionQueues {
            queues: vec![
                worldwake_core::ContentionQueue::default(),
                worldwake_core::ContentionQueue::default(),
                worldwake_core::ContentionQueue::default(),
            ],
        };

        let (agent, near_source, remote_source) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let near_source = txn.create_entity(EntityKind::Facility);
            let remote_source = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(agent, near_place).unwrap();
            txn.set_ground_location(near_source, near_place).unwrap();
            txn.set_ground_location(remote_source, remote_place)
                .unwrap();
            txn.set_component_resource_extraction_queues(near_source, queues.clone())
                .unwrap();
            txn.set_component_resource_extraction_queues(remote_source, queues.clone())
                .unwrap();
            commit_txn(txn);
            (agent, near_source, remote_source)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        // Co-located source: queues are visible (FND-14A — concrete
        // physical occupancy of a co-located entity).
        assert_eq!(
            FacilityBeliefView::resource_extraction_queues(&view, near_source),
            Some(queues.clone())
        );
        assert_eq!(
            GoalBeliefView::resource_extraction_queues(&view, near_source),
            Some(queues.clone())
        );

        // Remote source: queues are gated off.
        assert_eq!(
            FacilityBeliefView::resource_extraction_queues(&view, remote_source),
            None
        );
        assert_eq!(
            GoalBeliefView::resource_extraction_queues(&view, remote_source),
            None
        );
    }

    #[test]
    fn contention_accessors_require_local_visibility() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let near_place = places[0];
        let remote_place = world.topology().neighbors(near_place)[0];
        let action = ActionDefId(77);
        let queued_at = Tick(7);
        let reservation_range = TickRange::new(Tick(10), Tick(14)).unwrap();
        let query_range = TickRange::new(Tick(12), Tick(16)).unwrap();

        let (agent, near_source, remote_source, near_facility, remote_facility) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let reserver = txn.create_agent("Bryn", ControlSource::Ai).unwrap();
            let near_source = txn.create_entity(EntityKind::Facility);
            let remote_source = txn.create_entity(EntityKind::Facility);
            let near_facility = txn.create_entity(EntityKind::Facility);
            let remote_facility = txn.create_entity(EntityKind::Facility);

            txn.set_ground_location(agent, near_place).unwrap();
            txn.set_ground_location(reserver, remote_place).unwrap();
            txn.set_ground_location(near_source, near_place).unwrap();
            txn.set_ground_location(remote_source, remote_place)
                .unwrap();
            txn.set_ground_location(near_facility, near_place).unwrap();
            txn.set_ground_location(remote_facility, remote_place)
                .unwrap();

            for source in [near_source, remote_source] {
                txn.set_component_resource_extraction_queues(
                    source,
                    ResourceExtractionQueues {
                        queues: vec![queue_waiting_for(agent, action, queued_at)],
                    },
                )
                .unwrap();
            }
            for facility in [near_facility, remote_facility] {
                txn.set_component_contention_queue(
                    facility,
                    queue_waiting_for(agent, action, queued_at),
                )
                .unwrap();
                txn.try_reserve(facility, reserver, reservation_range)
                    .unwrap();
            }

            commit_txn(txn);
            (
                agent,
                near_source,
                remote_source,
                near_facility,
                remote_facility,
            )
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(TemporalBeliefView::actor_can_claim_extraction_slot(
            &view,
            near_source,
            agent
        ));
        assert!(TemporalBeliefView::has_extraction_queues(
            &view,
            near_source
        ));
        assert_eq!(
            TemporalBeliefView::facility_queue_join_tick(&view, near_facility, agent),
            Some(queued_at)
        );
        assert!(TemporalBeliefView::reservation_conflicts(
            &view,
            near_facility,
            query_range
        ));
        assert_eq!(
            TemporalBeliefView::reservation_ranges(&view, near_facility),
            vec![reservation_range]
        );

        assert!(!TemporalBeliefView::actor_can_claim_extraction_slot(
            &view,
            remote_source,
            agent
        ));
        assert!(!TemporalBeliefView::has_extraction_queues(
            &view,
            remote_source
        ));
        assert_eq!(
            TemporalBeliefView::facility_queue_join_tick(&view, remote_facility, agent),
            None
        );
        assert!(!TemporalBeliefView::reservation_conflicts(
            &view,
            remote_facility,
            query_range
        ));
        assert!(TemporalBeliefView::reservation_ranges(&view, remote_facility).is_empty());
    }

    fn queue_waiting_for(
        actor: EntityId,
        intended_action: ActionDefId,
        queued_at: Tick,
    ) -> ContentionQueue {
        ContentionQueue {
            next_ordinal: 1,
            waiting: BTreeMap::from([(
                0,
                ContentionWaiter {
                    actor,
                    intended_action,
                    queued_at,
                },
            )]),
            granted: None,
        }
    }
}
