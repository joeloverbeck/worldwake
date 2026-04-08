use crate::{
    ActionDuration, ActionPayload, DurationExpr, RecipeDefinition,
    action_semantics::consultation_duration_ticks,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDomain, AgentBeliefStore, BeliefConfidencePolicy, BelievedActivity, BelievedEntityState,
    BelievedInstitutionalClaim, CombatProfile, CommodityConsumableProfile, CommodityKind,
    CommodityTreatmentProfile, CommodityValuationProfile, ContentionGrant, DemandObservation,
    DriveThresholds, EffectiveRight, EntityId, EntityKind, ExpectationStore, HomeostaticNeeds,
    InTransitOnEdge, InstitutionalBeliefKey, InstitutionalBeliefRead, IntentionDispositionProfile,
    JusticeDispositionProfile, LastSeenMemory, LoadUnits, MerchandiseProfile, MetabolismProfile,
    OfficeData, PatrolProfile, PatrolRoute, Permille, PlaceTag, PlaceTagSet, PreferenceProfile,
    Quantity, RecipeId, RecipientKnowledgeStatus, RecordData, RecordedViolation, ResourceSource,
    RouteExperience, SocialObservation, SourceReliability, StockStoragePolicy, TellMemoryKey,
    TellProfile, TellTopic, Tick, TickRange, ToldBeliefMemory, TradeDispositionProfile,
    UniqueItemKind, UtilityProfile, ViolationDispositionProfile, WorkstationTag, Wound,
};

/// Narrow AI-facing surface for goal formation, pressure derivation, ranking, and explanation.
///
/// Classification:
/// - subjective reads: observed non-self state such as `effective_place`, `commodity_quantity`,
///   `corpse_entities_at`, `listed_sale_lots_at`, `seller_for_sale_lot`
/// - self-authoritative reads: self needs, wounds, recipes, inventory, load, profiles
/// - public structure reads: topology, place tags, workstation and source discovery
///
/// Deliberately excluded from this trait:
/// - queue and reservation helpers
/// - duration estimation
/// - broader affordance/runtime helpers used by snapshot/search code
pub trait GoalBeliefView {
    fn current_tick(&self) -> Tick {
        Tick(0)
    }
    fn is_alive(&self, entity: EntityId) -> bool;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn locally_observed_is_dead(&self, agent: EntityId, entity: EntityId) -> bool {
        let _ = agent;
        self.is_dead(entity)
    }
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn locally_observed_entities_at(&self, agent: EntityId, place: EntityId) -> Vec<EntityId> {
        let _ = agent;
        self.entities_at(place)
    }
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        let _ = agent;
        None
    }
    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        let _ = agent;
        Vec::new()
    }
    fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
        let _ = agent;
        Vec::new()
    }
    fn factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        let _ = entity;
        Vec::new()
    }
    fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        let _ = entity;
        Vec::new()
    }
    fn locally_observed_bandit_camp_faction_at(
        &self,
        agent: EntityId,
        place: EntityId,
    ) -> Option<EntityId> {
        let _ = (agent, place);
        None
    }
    fn believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity> {
        let _ = entity;
        None
    }
    fn agents_active_at(
        &self,
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
    ) -> Vec<EntityId> {
        let _ = (place, domain, target);
        Vec::new()
    }
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
    fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
        let _ = recipe;
        None
    }
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn locally_observed_commodity_quantity(
        &self,
        agent: EntityId,
        holder: EntityId,
        kind: CommodityKind,
    ) -> Quantity {
        let _ = agent;
        self.commodity_quantity(holder, kind)
    }
    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity;
    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId>;
    fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        let _ = faction;
        None
    }
    fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<NonZeroU32> {
        let _ = faction;
        None
    }
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        let _ = (actor, entity);
        Vec::new()
    }
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn stock_storage_policy(&self, facility: EntityId) -> Option<StockStoragePolicy> {
        let _ = facility;
        None
    }
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
    fn observation_fidelity(&self, agent: EntityId) -> Permille {
        let _ = agent;
        Permille::new_unchecked(1000)
    }
    fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
        let _ = agent;
        None
    }
    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        let _ = agent;
        None
    }
    fn pursuit_profile(&self, agent: EntityId) -> Option<worldwake_core::PursuitProfile> {
        let _ = agent;
        None
    }
    fn epistemic_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::EpistemicDispositionProfile> {
        let _ = agent;
        None
    }
    fn theft_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::TheftDispositionProfile> {
        let _ = agent;
        None
    }
    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
        let _ = agent;
        None
    }
    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        let _ = agent;
        None
    }
    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        let _ = agent;
        Vec::new()
    }
    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<ToldBeliefMemory> {
        let _ = (actor, counterparty, topic);
        None
    }
    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<RecipientKnowledgeStatus> {
        let _ = (actor, counterparty, topic);
        None
    }
    fn ask_witness_memory(
        &self,
        actor: EntityId,
        key: &worldwake_core::AskWitnessMemoryKey,
    ) -> Option<worldwake_core::AskWitnessMemory> {
        let _ = (actor, key);
        None
    }
    fn courage(&self, agent: EntityId) -> Option<Permille> {
        let _ = agent;
        None
    }
    fn violation_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<ViolationDispositionProfile> {
        let _ = agent;
        None
    }
    fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
        let _ = agent;
        Vec::new()
    }
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
    fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
        let _ = agent;
        None
    }
    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        let _ = agent;
        None
    }
    fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
        let _ = agent;
        None
    }
    fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
        let _ = agent;
        None
    }
    fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
        let _ = agent;
        None
    }
    fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
        let _ = agent;
        None
    }
    fn utility_profile(&self, agent: EntityId) -> Option<UtilityProfile> {
        let _ = agent;
        None
    }
    fn wounds(&self, agent: EntityId) -> Vec<Wound>;
    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId>;
    fn has_sale_listing(&self, lot: EntityId) -> bool {
        let _ = lot;
        false
    }
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn record_data(&self, record: EntityId) -> Option<RecordData> {
        let _ = record;
        None
    }
    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        let _ = office;
        None
    }
    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = office;
        InstitutionalBeliefRead::Unknown
    }
    fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        let _ = office;
        InstitutionalBeliefRead::Unknown
    }
    fn believed_membership(
        &self,
        faction: EntityId,
        member: EntityId,
    ) -> InstitutionalBeliefRead<bool> {
        let _ = (faction, member);
        InstitutionalBeliefRead::Unknown
    }
    fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = faction;
        InstitutionalBeliefRead::Unknown
    }
    fn offices_contested_by(&self, claimant: EntityId) -> Vec<EntityId> {
        let _ = claimant;
        Vec::new()
    }
    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        let _ = (subject, target);
        None
    }
    fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = (office, supporter);
        InstitutionalBeliefRead::Unknown
    }
    fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        let _ = office;
        Vec::new()
    }
    /// Return the raw institutional belief claims for a key, with provenance.
    /// Default returns empty (backward compatible).
    fn institutional_belief_claims(
        &self,
        agent: EntityId,
        key: InstitutionalBeliefKey,
    ) -> Vec<BelievedInstitutionalClaim> {
        let _ = (agent, key);
        Vec::new()
    }
}

pub trait ControlBeliefView {
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        let _ = (actor, entity);
        Vec::new()
    }
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
}

pub trait EntityBeliefView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn locally_observed_is_dead(&self, agent: EntityId, entity: EntityId) -> bool {
        let _ = agent;
        self.is_dead(entity)
    }
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
        let _ = faction;
        None
    }
    fn bandit_camp_establishment_ticks(&self, faction: EntityId) -> Option<NonZeroU32> {
        let _ = faction;
        None
    }
    fn is_dead(&self, entity: EntityId) -> bool {
        !self.is_alive(entity)
    }
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
}

pub trait ProfileBeliefView {
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile>;
    fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
        let _ = agent;
        None
    }
    fn utility_profile(&self, agent: EntityId) -> Option<UtilityProfile> {
        let _ = agent;
        None
    }
}

pub trait SpatialBeliefView {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn is_in_transit(&self, entity: EntityId) -> bool;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn locally_observed_entities_at(&self, agent: EntityId, place: EntityId) -> Vec<EntityId> {
        let _ = agent;
        self.entities_at(place)
    }
    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId>;
    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        let _ = (place, tag);
        false
    }
    fn place_has_any_tag_in(&self, place: EntityId, tag_set: PlaceTagSet) -> bool {
        PlaceTag::ALL
            .iter()
            .any(|tag| tag_set.contains(*tag) && self.place_has_tag(place, *tag))
    }
    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        let _ = agent;
        None
    }
    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        let _ = agent;
        None
    }
    fn route_exists(&self, from: EntityId, to: EntityId) -> bool;
    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
}

pub trait TemporalBeliefView {
    fn current_tick(&self) -> Tick {
        Tick(0)
    }
    fn has_contention_policy(&self, entity: EntityId) -> bool {
        let _ = entity;
        false
    }
    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        let _ = (facility, actor);
        None
    }
    fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
        let _ = facility;
        None
    }
    fn contention_queue_is_full(&self, entity: EntityId) -> bool {
        let _ = entity;
        false
    }
    fn facility_queue_join_tick(&self, facility: EntityId, actor: EntityId) -> Option<Tick> {
        let _ = (facility, actor);
        None
    }
    fn facility_queue_patience_ticks(&self, agent: EntityId) -> Option<NonZeroU32> {
        let _ = agent;
        None
    }
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange>;
    fn estimate_duration(
        &self,
        actor: EntityId,
        duration: &DurationExpr,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Option<ActionDuration>;
}

pub trait InventoryBeliefView {
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
        let _ = recipe;
        None
    }
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn locally_observed_commodity_quantity(
        &self,
        agent: EntityId,
        holder: EntityId,
        kind: CommodityKind,
    ) -> Quantity {
        let _ = agent;
        self.commodity_quantity(holder, kind)
    }
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
}

pub trait CombatBeliefView {
    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile>;
    fn courage(&self, agent: EntityId) -> Option<Permille> {
        let _ = agent;
        None
    }
    fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
        let _ = agent;
        None
    }
    fn wounds(&self, agent: EntityId) -> Vec<Wound>;
    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        self.visible_hostiles_for(agent)
    }
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
        let _ = agent;
        None
    }
    fn pursuit_profile(&self, agent: EntityId) -> Option<worldwake_core::PursuitProfile> {
        let _ = agent;
        None
    }
    fn has_wounds(&self, entity: EntityId) -> bool;
}

pub trait EconomicBeliefView {
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile>;
    fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
        let _ = agent;
        None
    }
    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity;
    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId>;
    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId>;
    fn has_sale_listing(&self, lot: EntityId) -> bool {
        let _ = lot;
        false
    }
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
}

pub trait FacilityBeliefView {
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn stock_storage_policy(&self, facility: EntityId) -> Option<StockStoragePolicy> {
        let _ = facility;
        None
    }
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
}

/// Richer AI/runtime-facing surface for planning snapshots, affordance search, revalidation,
/// failure handling, and duration estimation.
///
/// This trait is intentionally broader than `GoalBeliefView`. Callers should only depend on it
/// when they truly need runtime-only helpers such as reservations, queue state, or duration
/// estimation.
pub trait RuntimeBeliefView:
    ControlBeliefView
    + EntityBeliefView
    + ProfileBeliefView
    + SpatialBeliefView
    + TemporalBeliefView
    + InventoryBeliefView
    + CombatBeliefView
    + EconomicBeliefView
    + FacilityBeliefView
{
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        let _ = agent;
        None
    }
    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        let _ = agent;
        Vec::new()
    }
    fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
        let _ = agent;
        Vec::new()
    }
    fn factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        let _ = entity;
        Vec::new()
    }
    fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
        let _ = entity;
        Vec::new()
    }
    fn locally_observed_bandit_camp_faction_at(
        &self,
        agent: EntityId,
        place: EntityId,
    ) -> Option<EntityId> {
        let _ = (agent, place);
        None
    }
    fn believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity> {
        let _ = entity;
        None
    }
    fn agents_active_at(
        &self,
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
    ) -> Vec<EntityId> {
        let _ = (place, domain, target);
        Vec::new()
    }
    fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
    fn observation_fidelity(&self, agent: EntityId) -> Permille {
        let _ = agent;
        Permille::new_unchecked(1000)
    }
    fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
        let _ = agent;
        None
    }
    fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
        let _ = agent;
        None
    }
    fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
        let _ = agent;
        None
    }
    fn epistemic_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::EpistemicDispositionProfile> {
        let _ = agent;
        None
    }
    fn theft_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<worldwake_core::TheftDispositionProfile> {
        let _ = agent;
        None
    }
    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
        let _ = agent;
        None
    }
    fn intention_disposition_profile(&self, agent: EntityId)
    -> Option<IntentionDispositionProfile>;
    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        let _ = agent;
        None
    }
    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        let _ = agent;
        Vec::new()
    }
    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<ToldBeliefMemory> {
        let _ = (actor, counterparty, topic);
        None
    }
    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        topic: &TellTopic,
    ) -> Option<RecipientKnowledgeStatus> {
        let _ = (actor, counterparty, topic);
        None
    }
    fn ask_witness_memory(
        &self,
        actor: EntityId,
        key: &worldwake_core::AskWitnessMemoryKey,
    ) -> Option<worldwake_core::AskWitnessMemory> {
        let _ = (actor, key);
        None
    }
    fn violation_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<ViolationDispositionProfile> {
        let _ = agent;
        None
    }
    fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
        let _ = agent;
        Vec::new()
    }
    fn record_data(&self, record: EntityId) -> Option<RecordData> {
        let _ = record;
        None
    }
    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        let _ = office;
        None
    }
    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = office;
        InstitutionalBeliefRead::Unknown
    }
    fn believed_force_controller(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
        let _ = office;
        InstitutionalBeliefRead::Unknown
    }
    fn believed_membership(
        &self,
        faction: EntityId,
        member: EntityId,
    ) -> InstitutionalBeliefRead<bool> {
        let _ = (faction, member);
        InstitutionalBeliefRead::Unknown
    }
    fn believed_faction_rally_point(
        &self,
        faction: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = faction;
        InstitutionalBeliefRead::Unknown
    }
    fn offices_contested_by(&self, claimant: EntityId) -> Vec<EntityId> {
        let _ = claimant;
        Vec::new()
    }
    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        let _ = (subject, target);
        None
    }
    fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        let _ = (office, supporter);
        InstitutionalBeliefRead::Unknown
    }
    fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        let _ = office;
        Vec::new()
    }
    fn institutional_belief_claims(
        &self,
        agent: EntityId,
        key: InstitutionalBeliefKey,
    ) -> Vec<BelievedInstitutionalClaim> {
        let _ = (agent, key);
        Vec::new()
    }
}

#[macro_export]
macro_rules! impl_goal_belief_view {
    ($ty:ty) => {
        impl $crate::GoalBeliefView for $ty {
            fn current_tick(&self) -> worldwake_core::Tick {
                $crate::TemporalBeliefView::current_tick(self)
            }

            fn is_alive(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::EntityBeliefView::is_alive(self, entity)
            }

            fn is_dead(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::EntityBeliefView::is_dead(self, entity)
            }

            fn locally_observed_is_dead(
                &self,
                agent: worldwake_core::EntityId,
                entity: worldwake_core::EntityId,
            ) -> bool {
                $crate::EntityBeliefView::locally_observed_is_dead(self, agent, entity)
            }

            fn entity_kind(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityKind> {
                $crate::EntityBeliefView::entity_kind(self, entity)
            }

            fn effective_place(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::SpatialBeliefView::effective_place(self, entity)
            }

            fn entities_at(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::SpatialBeliefView::entities_at(self, place)
            }

            fn locally_observed_entities_at(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::SpatialBeliefView::locally_observed_entities_at(self, agent, place)
            }

            fn direct_possessions(
                &self,
                holder: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::InventoryBeliefView::direct_possessions(self, holder)
            }

            fn known_entity_beliefs(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<(
                worldwake_core::EntityId,
                worldwake_core::BelievedEntityState,
            )> {
                $crate::RuntimeBeliefView::known_entity_beliefs(self, agent)
            }

            fn agent_belief_store(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<&worldwake_core::AgentBeliefStore> {
                $crate::RuntimeBeliefView::agent_belief_store(self, agent)
            }

            fn known_social_observations(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::SocialObservation> {
                $crate::RuntimeBeliefView::known_social_observations(self, agent)
            }

            fn known_institutional_beliefs(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::BelievedInstitutionalClaim> {
                $crate::RuntimeBeliefView::known_institutional_beliefs(self, agent)
            }

            fn factions_of(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::factions_of(self, entity)
            }

            fn bandit_factions_of(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::bandit_factions_of(self, entity)
            }

            fn locally_observed_bandit_camp_faction_at(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::locally_observed_bandit_camp_faction_at(
                    self, agent, place,
                )
            }

            fn believed_activity_of(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<&worldwake_core::BelievedActivity> {
                $crate::RuntimeBeliefView::believed_activity_of(self, entity)
            }

            fn agents_active_at(
                &self,
                place: worldwake_core::EntityId,
                domain: worldwake_core::ActionDomain,
                target: Option<worldwake_core::EntityId>,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::RuntimeBeliefView::agents_active_at(self, place, domain, target)
            }

            fn adjacent_places_with_travel_ticks(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<(worldwake_core::EntityId, std::num::NonZeroU32)> {
                $crate::SpatialBeliefView::adjacent_places_with_travel_ticks(self, place)
            }

            fn knows_recipe(
                &self,
                actor: worldwake_core::EntityId,
                recipe: worldwake_core::RecipeId,
            ) -> bool {
                $crate::InventoryBeliefView::knows_recipe(self, actor, recipe)
            }

            fn known_recipes(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::RecipeId> {
                $crate::InventoryBeliefView::known_recipes(self, agent)
            }

            fn recipe_definition(
                &self,
                recipe: worldwake_core::RecipeId,
            ) -> Option<$crate::RecipeDefinition> {
                $crate::InventoryBeliefView::recipe_definition(self, recipe)
            }

            fn unique_item_count(
                &self,
                holder: worldwake_core::EntityId,
                kind: worldwake_core::UniqueItemKind,
            ) -> u32 {
                $crate::InventoryBeliefView::unique_item_count(self, holder, kind)
            }

            fn commodity_quantity(
                &self,
                holder: worldwake_core::EntityId,
                kind: worldwake_core::CommodityKind,
            ) -> worldwake_core::Quantity {
                $crate::InventoryBeliefView::commodity_quantity(self, holder, kind)
            }

            fn locally_observed_commodity_quantity(
                &self,
                agent: worldwake_core::EntityId,
                holder: worldwake_core::EntityId,
                kind: worldwake_core::CommodityKind,
            ) -> worldwake_core::Quantity {
                $crate::InventoryBeliefView::locally_observed_commodity_quantity(
                    self, agent, holder, kind,
                )
            }

            fn controlled_commodity_quantity_at_place(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> worldwake_core::Quantity {
                $crate::EconomicBeliefView::controlled_commodity_quantity_at_place(
                    self, agent, place, commodity,
                )
            }

            fn local_controlled_lots_for(
                &self,
                agent: worldwake_core::EntityId,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::EconomicBeliefView::local_controlled_lots_for(self, agent, place, commodity)
            }

            fn bandit_flee_wound_threshold(
                &self,
                faction: worldwake_core::EntityId,
            ) -> Option<worldwake_core::Permille> {
                $crate::EntityBeliefView::bandit_flee_wound_threshold(self, faction)
            }

            fn item_lot_commodity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::CommodityKind> {
                $crate::InventoryBeliefView::item_lot_commodity(self, entity)
            }

            fn item_lot_consumable_profile(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::CommodityConsumableProfile> {
                $crate::InventoryBeliefView::item_lot_consumable_profile(self, entity)
            }

            fn direct_container(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::InventoryBeliefView::direct_container(self, entity)
            }

            fn direct_possessor(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::InventoryBeliefView::direct_possessor(self, entity)
            }

            fn believed_owner_of(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::ControlBeliefView::believed_owner_of(self, entity)
            }

            fn believed_rights(
                &self,
                actor: worldwake_core::EntityId,
                entity: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EffectiveRight> {
                $crate::ControlBeliefView::believed_rights(self, actor, entity)
            }

            fn workstation_tag(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::WorkstationTag> {
                $crate::FacilityBeliefView::workstation_tag(self, entity)
            }

            fn resource_source(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::ResourceSource> {
                $crate::FacilityBeliefView::resource_source(self, entity)
            }

            fn resource_sources_at(
                &self,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::FacilityBeliefView::resource_sources_at(self, place, commodity)
            }

            fn matching_workstations_at(
                &self,
                place: worldwake_core::EntityId,
                tag: worldwake_core::WorkstationTag,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::FacilityBeliefView::matching_workstations_at(self, place, tag)
            }

            fn has_production_job(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::FacilityBeliefView::has_production_job(self, entity)
            }

            fn can_control(
                &self,
                actor: worldwake_core::EntityId,
                entity: worldwake_core::EntityId,
            ) -> bool {
                $crate::ControlBeliefView::can_control(self, actor, entity)
            }

            fn stock_storage_policy(
                &self,
                facility: worldwake_core::EntityId,
            ) -> Option<worldwake_core::StockStoragePolicy> {
                $crate::FacilityBeliefView::stock_storage_policy(self, facility)
            }

            fn carry_capacity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::LoadUnits> {
                $crate::InventoryBeliefView::carry_capacity(self, entity)
            }

            fn load_of_entity(
                &self,
                entity: worldwake_core::EntityId,
            ) -> Option<worldwake_core::LoadUnits> {
                $crate::InventoryBeliefView::load_of_entity(self, entity)
            }

            fn is_incapacitated(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::EntityBeliefView::is_incapacitated(self, entity)
            }

            fn has_wounds(&self, entity: worldwake_core::EntityId) -> bool {
                $crate::CombatBeliefView::has_wounds(self, entity)
            }

            fn homeostatic_needs(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::HomeostaticNeeds> {
                $crate::ProfileBeliefView::homeostatic_needs(self, agent)
            }

            fn drive_thresholds(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::DriveThresholds> {
                $crate::ProfileBeliefView::drive_thresholds(self, agent)
            }

            fn belief_confidence_policy(
                &self,
                agent: worldwake_core::EntityId,
            ) -> worldwake_core::BeliefConfidencePolicy {
                $crate::RuntimeBeliefView::belief_confidence_policy(self, agent)
            }

            fn observation_fidelity(
                &self,
                agent: worldwake_core::EntityId,
            ) -> worldwake_core::Permille {
                $crate::RuntimeBeliefView::observation_fidelity(self, agent)
            }

            fn patrol_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::PatrolProfile> {
                $crate::CombatBeliefView::patrol_profile(self, agent)
            }

            fn patrol_route(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::PatrolRoute> {
                $crate::SpatialBeliefView::patrol_route(self, agent)
            }

            fn pursuit_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::PursuitProfile> {
                $crate::CombatBeliefView::pursuit_profile(self, agent)
            }

            fn epistemic_disposition_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EpistemicDispositionProfile> {
                $crate::RuntimeBeliefView::epistemic_disposition_profile(self, agent)
            }

            fn theft_disposition_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::TheftDispositionProfile> {
                $crate::RuntimeBeliefView::theft_disposition_profile(self, agent)
            }

            fn justice_disposition_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::JusticeDispositionProfile> {
                $crate::RuntimeBeliefView::justice_disposition_profile(self, agent)
            }

            fn tell_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::TellProfile> {
                $crate::RuntimeBeliefView::tell_profile(self, agent)
            }

            fn told_belief_memories(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<(
                worldwake_core::TellMemoryKey,
                worldwake_core::ToldBeliefMemory,
            )> {
                $crate::RuntimeBeliefView::told_belief_memories(self, agent)
            }

            fn told_belief_memory(
                &self,
                actor: worldwake_core::EntityId,
                counterparty: worldwake_core::EntityId,
                topic: &worldwake_core::TellTopic,
            ) -> Option<worldwake_core::ToldBeliefMemory> {
                $crate::RuntimeBeliefView::told_belief_memory(self, actor, counterparty, topic)
            }

            fn recipient_knowledge_status(
                &self,
                actor: worldwake_core::EntityId,
                counterparty: worldwake_core::EntityId,
                topic: &worldwake_core::TellTopic,
            ) -> Option<worldwake_core::RecipientKnowledgeStatus> {
                $crate::RuntimeBeliefView::recipient_knowledge_status(
                    self,
                    actor,
                    counterparty,
                    topic,
                )
            }

            fn ask_witness_memory(
                &self,
                actor: worldwake_core::EntityId,
                key: &worldwake_core::AskWitnessMemoryKey,
            ) -> Option<worldwake_core::AskWitnessMemory> {
                $crate::RuntimeBeliefView::ask_witness_memory(self, actor, key)
            }

            fn courage(&self, agent: worldwake_core::EntityId) -> Option<worldwake_core::Permille> {
                $crate::CombatBeliefView::courage(self, agent)
            }

            fn violation_disposition_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::ViolationDispositionProfile> {
                $crate::RuntimeBeliefView::violation_disposition_profile(self, agent)
            }

            fn active_violation_records(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::RecordedViolation> {
                $crate::RuntimeBeliefView::active_violation_records(self, agent)
            }

            fn merchandise_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::MerchandiseProfile> {
                $crate::EconomicBeliefView::merchandise_profile(self, agent)
            }

            fn commodity_valuation_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::CommodityValuationProfile> {
                $crate::EconomicBeliefView::commodity_valuation_profile(self, agent)
            }

            fn route_experience(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::RouteExperience> {
                $crate::SpatialBeliefView::route_experience(self, agent)
            }

            fn source_reliability(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::SourceReliability> {
                $crate::RuntimeBeliefView::source_reliability(self, agent)
            }

            fn preference_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::PreferenceProfile> {
                $crate::ProfileBeliefView::preference_profile(self, agent)
            }

            fn expectation_store(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::ExpectationStore> {
                $crate::RuntimeBeliefView::expectation_store(self, agent)
            }

            fn last_seen_memory(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::LastSeenMemory> {
                $crate::RuntimeBeliefView::last_seen_memory(self, agent)
            }

            fn utility_profile(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Option<worldwake_core::UtilityProfile> {
                $crate::ProfileBeliefView::utility_profile(self, agent)
            }

            fn wounds(&self, agent: worldwake_core::EntityId) -> Vec<worldwake_core::Wound> {
                $crate::CombatBeliefView::wounds(self, agent)
            }

            fn hostile_targets_of(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::CombatBeliefView::hostile_targets_of(self, agent)
            }

            fn visible_hostiles_for(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::CombatBeliefView::visible_hostiles_for(self, agent)
            }

            fn current_attackers_of(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::CombatBeliefView::current_attackers_of(self, agent)
            }

            fn listed_sale_lots_at(
                &self,
                place: worldwake_core::EntityId,
                commodity: worldwake_core::CommodityKind,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::EconomicBeliefView::listed_sale_lots_at(self, place, commodity)
            }

            fn seller_for_sale_lot(
                &self,
                lot: worldwake_core::EntityId,
            ) -> Option<worldwake_core::EntityId> {
                $crate::EconomicBeliefView::seller_for_sale_lot(self, lot)
            }

            fn has_sale_listing(&self, lot: worldwake_core::EntityId) -> bool {
                $crate::EconomicBeliefView::has_sale_listing(self, lot)
            }

            fn demand_memory(
                &self,
                agent: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::DemandObservation> {
                $crate::EconomicBeliefView::demand_memory(self, agent)
            }

            fn corpse_entities_at(
                &self,
                place: worldwake_core::EntityId,
            ) -> Vec<worldwake_core::EntityId> {
                $crate::EntityBeliefView::corpse_entities_at(self, place)
            }

            fn record_data(
                &self,
                record: worldwake_core::EntityId,
            ) -> Option<worldwake_core::RecordData> {
                $crate::RuntimeBeliefView::record_data(self, record)
            }

            fn office_data(
                &self,
                office: worldwake_core::EntityId,
            ) -> Option<worldwake_core::OfficeData> {
                $crate::RuntimeBeliefView::office_data(self, office)
            }

            fn believed_office_holder(
                &self,
                office: worldwake_core::EntityId,
            ) -> worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>> {
                $crate::RuntimeBeliefView::believed_office_holder(self, office)
            }

            fn believed_force_controller(
                &self,
                office: worldwake_core::EntityId,
            ) -> worldwake_core::InstitutionalBeliefRead<(Option<worldwake_core::EntityId>, bool)>
            {
                $crate::RuntimeBeliefView::believed_force_controller(self, office)
            }

            fn believed_membership(
                &self,
                faction: worldwake_core::EntityId,
                member: worldwake_core::EntityId,
            ) -> worldwake_core::InstitutionalBeliefRead<bool> {
                $crate::RuntimeBeliefView::believed_membership(self, faction, member)
            }

            fn believed_faction_rally_point(
                &self,
                faction: worldwake_core::EntityId,
            ) -> worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>> {
                $crate::RuntimeBeliefView::believed_faction_rally_point(self, faction)
            }

            fn loyalty_to(
                &self,
                subject: worldwake_core::EntityId,
                target: worldwake_core::EntityId,
            ) -> Option<worldwake_core::Permille> {
                $crate::RuntimeBeliefView::loyalty_to(self, subject, target)
            }

            fn believed_support_declaration(
                &self,
                office: worldwake_core::EntityId,
                supporter: worldwake_core::EntityId,
            ) -> worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>> {
                $crate::RuntimeBeliefView::believed_support_declaration(self, office, supporter)
            }

            fn believed_support_declarations_for_office(
                &self,
                office: worldwake_core::EntityId,
            ) -> Vec<(
                worldwake_core::EntityId,
                worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>>,
            )> {
                $crate::RuntimeBeliefView::believed_support_declarations_for_office(self, office)
            }

            fn institutional_belief_claims(
                &self,
                agent: worldwake_core::EntityId,
                key: worldwake_core::InstitutionalBeliefKey,
            ) -> Vec<worldwake_core::BelievedInstitutionalClaim> {
                $crate::RuntimeBeliefView::institutional_belief_claims(self, agent, key)
            }
        }
    };
}

#[must_use]
pub fn estimate_duration_from_beliefs(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    duration: &DurationExpr,
    targets: &[EntityId],
    payload: &ActionPayload,
) -> Option<ActionDuration> {
    match *duration {
        DurationExpr::Fixed(ticks) => Some(ActionDuration::new(ticks.get())),
        DurationExpr::ConsultRecord { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let record = view.record_data(target)?;
            let factor = view.consultation_speed_factor(actor)?;
            Some(ActionDuration::new(consultation_duration_ticks(
                record.consultation_ticks,
                factor,
            )))
        }
        DurationExpr::TargetConsumable { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let profile = view.item_lot_consumable_profile(target)?;
            Some(ActionDuration::new(
                profile.consumption_ticks_per_unit.get(),
            ))
        }
        DurationExpr::TravelToTarget { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let origin = view.effective_place(actor)?;
            view.adjacent_places_with_travel_ticks(origin)
                .into_iter()
                .find_map(|(adjacent, ticks)| {
                    (adjacent == target).then_some(ActionDuration::new(ticks.get()))
                })
        }
        DurationExpr::EscortRouteTravel => {
            let target = payload.as_escort_to_safety()?.destination;
            let origin = view.effective_place(actor)?;
            estimate_route_duration_from_beliefs(view, origin, target).map(ActionDuration::new)
        }
        DurationExpr::ActorMetabolism { kind } => {
            let profile = view.metabolism_profile(actor)?;
            let ticks = match kind {
                crate::MetabolismDurationKind::Toilet => profile.toilet_ticks.get(),
                crate::MetabolismDurationKind::Wash => profile.wash_ticks.get(),
            };
            Some(ActionDuration::new(ticks))
        }
        DurationExpr::ActorTradeDisposition => view
            .trade_disposition_profile(actor)
            .map(|profile| ActionDuration::new(profile.negotiation_round_ticks.get())),
        DurationExpr::ActorMarketPresence => view
            .trade_disposition_profile(actor)
            .map(|profile| ActionDuration::new(profile.market_presence_ticks.get())),
        DurationExpr::ActorPatrolProfile => view.patrol_profile(actor).map(|profile| {
            ActionDuration::new(crate::action_semantics::patrol_duration_ticks(&profile))
        }),
        DurationExpr::ActorTheftDisposition => view
            .theft_disposition_profile(actor)
            .map(|profile| ActionDuration::new(profile.steal_duration_ticks.get())),
        DurationExpr::ActorInvestigationDisposition => view
            .violation_disposition_profile(actor)
            .map(|profile| ActionDuration::new(profile.investigation_duration_ticks.get())),
        DurationExpr::ActorWitnessQueryDisposition => view
            .epistemic_disposition_profile(actor)
            .map(|profile| ActionDuration::new(profile.witness_query_duration_ticks.get())),
        DurationExpr::BanditCampEstablishmentProfile => payload
            .as_establish_camp()
            .and_then(|payload| view.bandit_camp_establishment_ticks(payload.faction))
            .map(|ticks| ActionDuration::new(ticks.get())),
        DurationExpr::ActorDefendStance => view
            .combat_profile(actor)
            .map(|profile| ActionDuration::new(profile.defend_stance_ticks.get())),
        DurationExpr::CombatWeapon => {
            let combat = payload.as_combat()?;
            match combat.weapon {
                worldwake_core::CombatWeaponRef::Unarmed => view
                    .combat_profile(actor)
                    .map(|profile| ActionDuration::new(profile.unarmed_attack_ticks.get())),
                worldwake_core::CombatWeaponRef::Commodity(kind) => kind
                    .spec()
                    .combat_weapon_profile
                    .map(|profile| ActionDuration::new(profile.attack_duration_ticks.get())),
            }
        }
        DurationExpr::TargetTreatment {
            target_index,
            commodity,
        } => {
            if view.commodity_quantity(actor, commodity) == Quantity(0) {
                return None;
            }
            let target = targets.get(usize::from(target_index)).copied()?;
            let wounds = view.wounds(target);
            if wounds.is_empty() {
                return None;
            }
            let CommodityTreatmentProfile {
                treatment_ticks_per_unit,
                severity_reduction_per_tick,
                ..
            } = commodity.spec().treatment_profile?;
            let wound_load = wounds.iter().fold(0u32, |acc, wound| {
                acc.saturating_add(u32::from(wound.severity.value()))
            });
            let severity_per_tick = u32::from(severity_reduction_per_tick.value()).max(1);
            let wound_ticks = wound_load.div_ceil(severity_per_tick).max(1);
            Some(ActionDuration::new(
                treatment_ticks_per_unit.get().max(wound_ticks),
            ))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct BeliefRouteQueueEntry {
    total_ticks: u32,
    place: EntityId,
}

impl Ord for BeliefRouteQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .total_ticks
            .cmp(&self.total_ticks)
            .then_with(|| self.place.cmp(&other.place))
    }
}

impl PartialOrd for BeliefRouteQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn estimate_route_duration_from_beliefs(
    view: &dyn RuntimeBeliefView,
    from: EntityId,
    to: EntityId,
) -> Option<u32> {
    if from == to {
        return Some(0);
    }

    let mut best = BTreeMap::new();
    best.insert(from, 0_u32);
    let mut frontier = BinaryHeap::new();
    frontier.push(BeliefRouteQueueEntry {
        total_ticks: 0,
        place: from,
    });

    while let Some(entry) = frontier.pop() {
        let known = *best.get(&entry.place)?;
        if entry.total_ticks != known {
            continue;
        }
        if entry.place == to {
            return Some(entry.total_ticks);
        }

        for (adjacent, ticks) in view.adjacent_places_with_travel_ticks(entry.place) {
            let candidate = entry.total_ticks.saturating_add(ticks.get());
            let should_replace = best
                .get(&adjacent)
                .is_none_or(|existing| candidate < *existing);
            if should_replace {
                best.insert(adjacent, candidate);
                frontier.push(BeliefRouteQueueEntry {
                    total_ticks: candidate,
                    place: adjacent,
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::estimate_duration_from_beliefs;
    use crate::{ActionPayload, DurationExpr, GoalBeliefView, PerAgentBeliefView};
    use worldwake_core::{
        AgentBeliefStore, CauseRef, CommodityConsumableProfile, CommodityKind, ControlSource,
        DemandObservation, DriveThresholds, EntityId, EntityKind, EventLog, HomeostaticNeeds,
        LoadUnits, PatrolProfile, Permille, Quantity, ResourceSource, Tick, UniqueItemKind,
        VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn, build_prototype_world,
    };

    struct StubGoalBeliefView;

    impl GoalBeliefView for StubGoalBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }

        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, std::num::NonZeroU32)> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
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

        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn merchandise_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MerchandiseProfile> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
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

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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
    fn estimate_duration_from_beliefs_returns_none_for_missing_investigation_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_agent_belief_store(actor, AgentBeliefStore::default())
                .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            estimate_duration_from_beliefs(
                &view,
                actor,
                &DurationExpr::ActorInvestigationDisposition,
                &[],
                &ActionPayload::None,
            ),
            None
        );
    }

    #[test]
    fn estimate_duration_from_beliefs_uses_patrol_profile_duration_contract() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_agent_belief_store(actor, AgentBeliefStore::default())
                .unwrap();
            txn.set_component_patrol_profile(
                actor,
                PatrolProfile {
                    base_dwell_ticks: 8,
                    dwell_vigilance_scale_ticks: 8,
                    vigilance: Permille::new(625).unwrap(),
                    route_adaptation_sensitivity: Permille::new(400).unwrap(),
                    patrol_motive_weight: Permille::new(550).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            estimate_duration_from_beliefs(
                &view,
                actor,
                &DurationExpr::ActorPatrolProfile,
                &[],
                &ActionPayload::None,
            ),
            Some(crate::ActionDuration::new(13))
        );
    }

    #[test]
    fn goal_belief_view_expectation_defaults_return_none() {
        let view = StubGoalBeliefView;
        let agent = EntityId {
            slot: 1,
            generation: 0,
        };

        assert_eq!(view.expectation_store(agent), None);
        assert_eq!(view.last_seen_memory(agent), None);
    }
}
