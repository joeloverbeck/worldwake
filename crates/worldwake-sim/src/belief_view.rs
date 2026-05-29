use crate::{
    ActionDuration, ActionPayload, DurationExpr, RecipeDefinition,
    action_semantics::consultation_duration_ticks,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDomain, AgentBeliefStore, AgentSchemaContextProfile, ArtifactPostingProfile,
    BeliefConfidencePolicy, BelievedActivity, BelievedEntityState, BelievedInstitutionalClaim,
    ClaimValue, CognitiveProfile, CombatProfile, CommodityConsumableProfile, CommodityKind,
    CommodityTreatmentProfile, CommodityValuationProfile, ContentionGrant, DemandObservation,
    DeprivationExposure, DisposalProfile, DiversificationProfile, DriveEscalationProfile,
    DriveThresholds, EffectiveRight, EntityBeliefAspect, EntityBeliefClaim, EntityId, EntityKind,
    ExpectationStore, ExplorationProfile, HomeostaticNeedId, HomeostaticNeeds, InTransitOnEdge,
    InstitutionalBeliefKey, InstitutionalBeliefRead, IntentionDispositionProfile,
    JusticeDispositionProfile, LastHarvestTrace, LastSeenMemory, LatrineFullness,
    LawAbidingProfile, LoadUnits, MerchandiseProfile, MetabolismProfile,
    ObligationExecutionTracker, ObligationSatiationProfile, ObservationOmissionLog, OfficeData,
    OfficePatrolDuty, PatrolProfile, PatrolRoute, PerceptionProfile, PerceptionSource, Permille,
    PlaceDirtiness, PlaceTag, PlaceTagSet, PortfolioWeightsProfile, PreferenceProfile, Quantity,
    RecipeId, RecipientKnowledgeStatus, RecordData, RecordKind, RecordedViolation,
    ResourceExtractionQueues, ResourceSource, RewardEncumbrance, RewardSource, RightKind,
    RiskWeightProfile, RouteExperience, RoutePreferenceProfile, SleepQualityProfile,
    SocialObservation, SourceReliability, StockStoragePolicy, SubstitutePreferences, TellMemoryKey,
    TellProfile, TellTopic, TestimonyTrustProfile, Tick, TickRange, ToldBeliefMemory,
    TradeDispositionProfile, UniqueItemKind, UtilityProfile, ViolationDispositionProfile,
    WashBasinState, WorkstationTag, Wound, effective_claim_confidence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefValue<T> {
    pub value: T,
    pub confidence: Permille,
    pub acquired_tick: Tick,
    pub claimed_event_tick: Option<Tick>,
    pub status: BeliefStatus,
}

#[derive(Clone, Debug)]
pub enum BeliefRead<T> {
    Unknown,
    Known(BeliefValue<T>),
    Stale(BeliefValue<T>),
}

impl<T> BeliefRead<T> {
    #[must_use]
    pub fn known_certain(value: T, tick: Tick) -> Self {
        Self::Known(BeliefValue {
            value,
            confidence: Permille::new(1000).unwrap(),
            acquired_tick: tick,
            claimed_event_tick: Some(tick),
            status: BeliefStatus::Certain,
        })
    }

    #[must_use]
    pub fn known_or_stale_value(self) -> Option<T> {
        match self {
            Self::Known(value) | Self::Stale(value) => Some(value.value),
            Self::Unknown => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObservedRead<T> {
    pub value: T,
    pub observed_tick: Tick,
    pub source: ObservationSource,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObservationSource {
    CoLocatedSameTick,
    BeliefStoreSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatus {
    Certain,
    Probable,
    Stale,
    Disputed,
    Contradicted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSet<T> {
    pub best: Option<BeliefValue<T>>,
    pub alternatives: Vec<BeliefValue<T>>,
}

impl<T> BeliefSet<T> {
    #[must_use]
    pub fn certain(value: T, acquired_tick: Tick) -> Self {
        Self {
            best: Some(BeliefValue {
                value,
                confidence: Permille::new(1000).unwrap(),
                acquired_tick,
                claimed_event_tick: Some(acquired_tick),
                status: BeliefStatus::Certain,
            }),
            alternatives: Vec::new(),
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            best: None,
            alternatives: Vec::new(),
        }
    }
}

#[doc(hidden)]
pub fn stale_default_value<T>(value: T) -> BeliefValue<T> {
    BeliefValue {
        value,
        confidence: Permille::ZERO,
        acquired_tick: Tick(0),
        claimed_event_tick: None,
        status: BeliefStatus::Stale,
    }
}

#[doc(hidden)]
pub fn belief_status_for_effective_confidence(effective: u16, threshold: Permille) -> BeliefStatus {
    let threshold = threshold.value();
    let certain_floor = threshold.saturating_mul(2).min(1000);
    if effective >= certain_floor {
        BeliefStatus::Certain
    } else if effective >= threshold {
        BeliefStatus::Probable
    } else {
        BeliefStatus::Stale
    }
}

#[doc(hidden)]
pub fn project_claim_into_belief_value<T: Copy>(
    claim: &EntityBeliefClaim,
    value: T,
    current_tick: Tick,
    threshold: Permille,
    policy: &BeliefConfidencePolicy,
) -> BeliefValue<T> {
    let effective = effective_claim_confidence(claim, current_tick, policy);
    let status = if claim.refuted_at_tick.is_some() {
        BeliefStatus::Contradicted
    } else {
        belief_status_for_effective_confidence(effective, threshold)
    };
    BeliefValue {
        value,
        confidence: Permille::new(effective).unwrap_or(Permille::ZERO),
        acquired_tick: claim.acquired_tick,
        claimed_event_tick: claim.claimed_event_tick,
        status,
    }
}

#[doc(hidden)]
pub fn claim_rank_key(
    claim: &EntityBeliefClaim,
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> (u16, Tick, worldwake_core::ClaimId) {
    (
        effective_claim_confidence(claim, current_tick, policy),
        claim.acquired_tick,
        claim.claim_id,
    )
}

#[doc(hidden)]
pub fn project_claims_into_belief_set<T, I>(
    claims: I,
    current_tick: Tick,
    threshold: Permille,
    policy: &BeliefConfidencePolicy,
) -> BeliefSet<T>
where
    T: Copy + Eq,
    I: IntoIterator<Item = (EntityBeliefClaim, T)>,
{
    let mut projected = claims
        .into_iter()
        .map(|(claim, value)| {
            (
                claim_rank_key(&claim, current_tick, policy),
                project_claim_into_belief_value(&claim, value, current_tick, threshold, policy),
            )
        })
        .collect::<Vec<_>>();

    if projected.is_empty() {
        return BeliefSet::empty();
    }

    let has_active_claim = projected
        .iter()
        .any(|(_, value)| value.status != BeliefStatus::Contradicted);
    if has_active_claim {
        projected.retain(|(_, value)| value.status != BeliefStatus::Contradicted);
    }

    projected.sort_by_key(|(rank, _)| *rank);
    let (_, mut best) = projected
        .pop()
        .expect("non-empty projected claims should contain a best value");

    let alternatives = projected
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| value.value != best.value)
        .collect::<Vec<_>>();

    if !alternatives.is_empty() && best.status != BeliefStatus::Contradicted {
        best.status = BeliefStatus::Disputed;
    }

    BeliefSet {
        best: Some(best),
        alternatives,
    }
}

#[doc(hidden)]
pub fn location_claim_value(claim: &EntityBeliefClaim) -> Option<Option<EntityId>> {
    match (&claim.aspect, &claim.value) {
        (EntityBeliefAspect::Location, ClaimValue::Place(place)) => Some(*place),
        _ => None,
    }
}

#[doc(hidden)]
pub fn entity_claim_value(
    claim: &EntityBeliefClaim,
    aspect: EntityBeliefAspect,
) -> Option<Option<EntityId>> {
    match (&claim.aspect, &claim.value) {
        (claim_aspect, ClaimValue::Entity(entity)) if *claim_aspect == aspect => Some(*entity),
        _ => None,
    }
}

#[doc(hidden)]
pub fn inventory_claim_value(claim: &EntityBeliefClaim, kind: CommodityKind) -> Option<Quantity> {
    match (&claim.aspect, &claim.value) {
        (EntityBeliefAspect::Inventory(claim_kind), ClaimValue::Quantity(quantity))
            if *claim_kind == kind =>
        {
            Some(*quantity)
        }
        _ => None,
    }
}

pub trait GoalSpatialBeliefView {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<BeliefValue<EntityId>> {
        let _ = (agent, place, kind);
        Vec::new()
    }
    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        let _ = agent;
        None
    }
    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        let _ = agent;
        None
    }
    fn office_patrol_duty(&self, agent: EntityId) -> Option<OfficePatrolDuty> {
        let _ = agent;
        None
    }
    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        let _ = (place, tag);
        false
    }
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
}

pub trait GoalTemporalBeliefView {
    fn current_tick(&self) -> Tick {
        Tick(0)
    }
}

pub trait GoalControlBeliefView {
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        let _ = (actor, entity);
        Vec::new()
    }
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
}

/// Narrow AI-facing surface for goal formation, pressure derivation, ranking, and explanation.
///
/// Classification:
/// - subjective reads: observed non-self state such as `effective_place`, `commodity_quantity`,
///   `corpse_entities_at`, `listed_sale_lots_at`, `seller_for_sale_lot`
/// - self-authoritative reads: self needs, wounds, recipes, inventory, load, profiles
/// - public structure reads: topology, workstation and source discovery, local institutional state
///
/// Deliberately excluded from this trait:
/// - queue and reservation helpers
/// - duration estimation
/// - broader affordance/runtime helpers used by snapshot/search code
pub trait GoalBeliefView: BelievedAuthorityView + LocalPhysicalObservationView {
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
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn entity_beliefs_sourced_from_witness(
        &self,
        agent: EntityId,
        witness: EntityId,
    ) -> Vec<(EntityId, BelievedEntityState)> {
        self.known_entity_beliefs(agent)
            .into_iter()
            .filter(|(_, belief)| {
                matches!(
                    belief.source,
                    PerceptionSource::Report { from, .. } if from == witness
                )
            })
            .collect()
    }
    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        let _ = agent;
        None
    }
    fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog> {
        self.agent_belief_store(agent)
            .map(|store| &store.observation_omission_log)
    }
    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        let _ = agent;
        Vec::new()
    }
    fn claim_confidence_threshold(&self, agent: EntityId) -> Permille {
        let _ = agent;
        Permille::ZERO
    }
    fn discrepancy_memory(&self, agent: EntityId) -> Option<&worldwake_core::DiscrepancyMemory> {
        let _ = agent;
        None
    }
    fn blocker_memory(&self, agent: EntityId) -> Option<&worldwake_core::BlockerMemory> {
        let _ = agent;
        None
    }
    fn repair_memory(&self, agent: EntityId) -> Option<&worldwake_core::RepairMemory> {
        let _ = agent;
        None
    }
    fn learned_opportunity_memory(
        &self,
        agent: EntityId,
    ) -> Option<&worldwake_core::LearnedOpportunityMemory> {
        let _ = agent;
        None
    }
    fn survey_memory(&self, agent: EntityId) -> Option<&worldwake_core::SurveyMemory> {
        let _ = agent;
        None
    }
    fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
        let _ = agent;
        Vec::new()
    }
    fn actor_lawful_reward_source_for_case(
        &self,
        actor: EntityId,
        accusation: &BelievedInstitutionalClaim,
    ) -> Option<RewardSource> {
        let _ = (actor, accusation);
        None
    }
    fn visible_reward_encumbrance(
        &self,
        actor: EntityId,
        office: EntityId,
    ) -> Option<&RewardEncumbrance> {
        let _ = (actor, office);
        None
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
    fn facility_wash_basin_state(&self, entity: EntityId) -> Option<WashBasinState> {
        let _ = entity;
        None
    }
    fn self_care_occupant(&self, entity: EntityId) -> Option<EntityId> {
        let _ = entity;
        None
    }
    fn rest_site_capacity(&self, place: EntityId) -> Option<NonZeroU32> {
        let _ = place;
        None
    }
    fn rest_site_occupant_count(&self, place: EntityId) -> Option<u32> {
        let _ = place;
        None
    }
    fn is_co_located_with_rest_site(&self, place: EntityId) -> bool {
        let _ = place;
        false
    }
    fn last_harvest_trace(&self, entity: EntityId) -> Option<LastHarvestTrace> {
        let _ = entity;
        None
    }
    fn resource_extraction_queues(&self, entity: EntityId) -> Option<ResourceExtractionQueues> {
        let _ = entity;
        None
    }
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        let _ = (place, tag);
        false
    }
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        let _ = agent;
        None
    }
    fn testimony_trust_profile(&self, agent: EntityId) -> Option<TestimonyTrustProfile> {
        let _ = agent;
        None
    }
    fn route_preference_profile(&self, agent: EntityId) -> Option<RoutePreferenceProfile> {
        let _ = agent;
        None
    }
    fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile {
        let _ = (agent, place);
        SleepQualityProfile::default()
    }
    fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness {
        let _ = (agent, place);
        PlaceDirtiness::default()
    }
    fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
        let _ = (agent, place);
        LatrineFullness::default()
    }
    fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
        let _ = (agent, basin);
        WashBasinState::default()
    }
    fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure> {
        let _ = agent;
        None
    }
    fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile> {
        let _ = agent;
        None
    }
    fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
        let _ = agent;
        None
    }
    fn exploration_profile(&self, agent: EntityId) -> Option<ExplorationProfile> {
        let _ = agent;
        None
    }
    fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile> {
        let _ = agent;
        None
    }
    fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick> {
        let _ = agent;
        None
    }
    fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
        let _ = (agent, need);
        0
    }
    fn obligation_satiation_profile(&self, agent: EntityId) -> ObligationSatiationProfile {
        let _ = agent;
        ObligationSatiationProfile::default()
    }
    fn obligation_execution_tracker(&self, agent: EntityId) -> ObligationExecutionTracker {
        let _ = agent;
        ObligationExecutionTracker::default()
    }
    fn cognitive_profile(&self, agent: EntityId) -> Option<CognitiveProfile> {
        let _ = agent;
        None
    }
    fn portfolio_weights_profile(&self, agent: EntityId) -> PortfolioWeightsProfile {
        let _ = agent;
        PortfolioWeightsProfile::default()
    }
    fn agent_schema_context_profile(&self, agent: EntityId) -> Option<AgentSchemaContextProfile> {
        let _ = agent;
        None
    }
    fn perception_profile(&self, agent: EntityId) -> Option<PerceptionProfile> {
        let _ = agent;
        None
    }
    fn risk_weight_profile(&self, agent: EntityId) -> Option<RiskWeightProfile> {
        let _ = agent;
        None
    }
    fn law_abiding_profile(&self, agent: EntityId) -> Option<LawAbidingProfile> {
        let _ = agent;
        None
    }
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
    fn office_patrol_duty(&self, agent: EntityId) -> Option<OfficePatrolDuty> {
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
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        let _ = agent;
        None
    }
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
    fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile> {
        let _ = agent;
        None
    }
    fn substitute_preferences(&self, agent: EntityId) -> Option<SubstitutePreferences> {
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
    fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
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
    fn believed_target_location(
        &self,
        agent: EntityId,
        target: EntityId,
    ) -> BeliefValue<Option<EntityId>> {
        let _ = (agent, target);
        stale_default_value(None)
    }
    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<BeliefValue<EntityId>> {
        let _ = (agent, place, kind);
        Vec::new()
    }
    fn believed_commodity_stock(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: CommodityKind,
    ) -> BeliefValue<Quantity> {
        let _ = (agent, place, kind);
        stale_default_value(Quantity(0))
    }
}

pub trait ControlBeliefView {
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        let _ = (actor, entity);
        Vec::new()
    }
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
}

pub trait LocalPhysicalObservationView {
    fn colocated_entities(&self, actor: EntityId) -> ObservedRead<Vec<EntityId>> {
        let _ = actor;
        ObservedRead {
            value: Vec::new(),
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_item_lot_quantity(&self, lot: EntityId) -> ObservedRead<Option<Quantity>> {
        let _ = lot;
        ObservedRead {
            value: None,
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_workstation_tag(&self, entity: EntityId) -> ObservedRead<Option<WorkstationTag>> {
        let _ = entity;
        ObservedRead {
            value: None,
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_resource_source(&self, entity: EntityId) -> ObservedRead<Option<ResourceSource>> {
        let _ = entity;
        ObservedRead {
            value: None,
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_container_contents(&self, container: EntityId) -> ObservedRead<Vec<EntityId>> {
        let _ = container;
        ObservedRead {
            value: Vec::new(),
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }

    fn observed_entity_kind(&self, entity: EntityId) -> ObservedRead<Option<EntityKind>> {
        let _ = entity;
        ObservedRead {
            value: None,
            observed_tick: Tick(0),
            source: ObservationSource::CoLocatedSameTick,
        }
    }
}

pub trait BelievedAuthorityView {
    fn believed_owner_of(&self, entity: EntityId) -> BeliefRead<EntityId> {
        let _ = entity;
        BeliefRead::Unknown
    }

    fn believed_holder_of(&self, entity: EntityId) -> BeliefRead<EntityId> {
        let _ = entity;
        BeliefRead::Unknown
    }

    fn believed_access_right(
        &self,
        actor: EntityId,
        target: EntityId,
    ) -> BeliefRead<EffectiveRight> {
        let _ = (actor, target);
        BeliefRead::Unknown
    }

    fn believed_jurisdiction(&self, place: EntityId) -> BeliefRead<EntityId> {
        let _ = place;
        BeliefRead::Unknown
    }

    fn believed_office_holder(&self, office: EntityId) -> BeliefRead<Option<EntityId>> {
        let _ = office;
        BeliefRead::Unknown
    }
}

/// Debug/observer access to authoritative world state.
///
/// `DebugWorldView` is deliberately outside the runtime belief-view trait
/// composition. Planner-facing code may read through `RuntimeBeliefView`, but
/// adding debug-world methods to that surface would pierce the FND-14A wall.
///
/// ```compile_fail
/// use worldwake_core::EntityId;
/// use worldwake_sim::{DebugWorldView, RuntimeBeliefView};
///
/// fn debug_read_from_runtime_view<T: RuntimeBeliefView + ?Sized>(
///     view: &T,
///     entity: EntityId,
/// ) {
///     let _ = view.world_owner_of(entity);
/// }
/// ```
#[cfg(any(debug_assertions, test))]
pub trait DebugWorldView {
    fn world_entity_state(&self, entity: EntityId) -> worldwake_core::EntityState;
    fn world_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_location_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_inventory_of(&self, entity: EntityId) -> Vec<EntityId>;
}

#[cfg(any(debug_assertions, test))]
impl DebugWorldView for &worldwake_core::World {
    fn world_entity_state(&self, entity: EntityId) -> worldwake_core::EntityState {
        worldwake_core::EntityState {
            kind: self.entity_kind(entity),
            place: self.effective_place(entity),
            alive: self.is_alive(entity),
            container: self.direct_container(entity),
            possessor: self.possessor_of(entity),
        }
    }

    fn world_owner_of(&self, entity: EntityId) -> Option<EntityId> {
        self.owner_of(entity)
    }

    fn world_location_of(&self, entity: EntityId) -> Option<EntityId> {
        self.effective_place(entity)
    }

    fn world_inventory_of(&self, entity: EntityId) -> Vec<EntityId> {
        self.possessions_of(entity)
    }
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
    fn believed_target_location(
        &self,
        agent: EntityId,
        target: EntityId,
    ) -> BeliefValue<Option<EntityId>> {
        let _ = (agent, target);
        stale_default_value(None)
    }
}

pub trait ProfileBeliefView {
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure> {
        let _ = agent;
        None
    }
    fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile> {
        let _ = agent;
        None
    }
    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile>;
    fn testimony_trust_profile(&self, agent: EntityId) -> Option<TestimonyTrustProfile> {
        let _ = agent;
        None
    }
    fn route_preference_profile(&self, agent: EntityId) -> Option<RoutePreferenceProfile> {
        let _ = agent;
        None
    }
    fn place_sleep_quality_profile(&self, agent: EntityId, place: EntityId) -> SleepQualityProfile {
        let _ = (agent, place);
        SleepQualityProfile::default()
    }
    fn place_dirtiness(&self, agent: EntityId, place: EntityId) -> PlaceDirtiness {
        let _ = (agent, place);
        PlaceDirtiness::default()
    }
    fn latrine_fullness(&self, agent: EntityId, place: EntityId) -> LatrineFullness {
        let _ = (agent, place);
        LatrineFullness::default()
    }
    fn wash_basin_state(&self, agent: EntityId, basin: EntityId) -> WashBasinState {
        let _ = (agent, basin);
        WashBasinState::default()
    }
    fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
        let _ = agent;
        None
    }
    fn exploration_profile(&self, agent: EntityId) -> Option<ExplorationProfile> {
        let _ = agent;
        None
    }
    fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile> {
        let _ = agent;
        None
    }
    fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick> {
        let _ = agent;
        None
    }
    fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
        let _ = (agent, need);
        0
    }
    fn obligation_satiation_profile(&self, agent: EntityId) -> ObligationSatiationProfile {
        let _ = agent;
        ObligationSatiationProfile::default()
    }
    fn obligation_execution_tracker(&self, agent: EntityId) -> ObligationExecutionTracker {
        let _ = agent;
        ObligationExecutionTracker::default()
    }
    fn cognitive_profile(&self, agent: EntityId) -> Option<CognitiveProfile> {
        let _ = agent;
        None
    }
    fn portfolio_weights_profile(&self, agent: EntityId) -> PortfolioWeightsProfile {
        let _ = agent;
        PortfolioWeightsProfile::default()
    }
    fn agent_schema_context_profile(&self, agent: EntityId) -> Option<AgentSchemaContextProfile> {
        let _ = agent;
        None
    }
    fn perception_profile(&self, agent: EntityId) -> Option<PerceptionProfile> {
        let _ = agent;
        None
    }
    fn risk_weight_profile(&self, agent: EntityId) -> Option<RiskWeightProfile> {
        let _ = agent;
        None
    }
    fn law_abiding_profile(&self, agent: EntityId) -> Option<LawAbidingProfile> {
        let _ = agent;
        None
    }
    fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
        let _ = agent;
        None
    }
    fn utility_profile(&self, agent: EntityId) -> Option<UtilityProfile> {
        let _ = agent;
        None
    }
    fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
        let _ = agent;
        None
    }
}

pub trait SpatialBeliefView {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn is_in_transit(&self, entity: EntityId) -> bool;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
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
    fn office_patrol_duty(&self, agent: EntityId) -> Option<OfficePatrolDuty> {
        let _ = agent;
        None
    }
    fn route_exists(&self, from: EntityId, to: EntityId) -> bool;
    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<BeliefValue<EntityId>> {
        let _ = (agent, place, kind);
        Vec::new()
    }
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
    /// Position of `actor` within whichever slot of `source`'s
    /// `ResourceExtractionQueues` they are currently waiting on. Returns
    /// `None` if the actor is not enqueued, or the source has no
    /// `ResourceExtractionQueues` registered. Mirrors
    /// `facility_queue_position` but reads the per-slot extraction
    /// substrate (FND-26 split between extraction-state and reservation-state).
    fn extraction_slot_queue_position(&self, source: EntityId, actor: EntityId) -> Option<u32> {
        let _ = (source, actor);
        None
    }
    /// True if `actor` currently holds a grant on any slot of `source`'s
    /// `ResourceExtractionQueues`.
    fn actor_holds_extraction_slot_grant(&self, source: EntityId, actor: EntityId) -> bool {
        let _ = (source, actor);
        false
    }
    /// True if `actor` can legally claim a slot of `source`'s
    /// `ResourceExtractionQueues` on their next harvest start request.
    /// A slot is claimable iff it has no active grant **and** either the
    /// slot has no waiters, or `actor` is the head of the waiting list
    /// (FND-26 FIFO semantics enforced by `grant_or_signal_full`).
    /// Returns `false` when the source has no `ResourceExtractionQueues`
    /// registered.
    fn actor_can_claim_extraction_slot(&self, source: EntityId, actor: EntityId) -> bool {
        let _ = (source, actor);
        false
    }
    /// True if `source` carries a `ResourceExtractionQueues` component
    /// with at least one slot. Identifies a resource source so that
    /// callers reasoning about harvest contention can avoid falling
    /// back to legacy temporal-reservation logic, which never applies
    /// to harvest sources (FND-26 separates extraction-state from
    /// reservation-state).
    fn has_extraction_queues(&self, source: EntityId) -> bool {
        let _ = source;
        false
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
    fn believed_commodity_stock(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: CommodityKind,
    ) -> BeliefValue<Quantity> {
        let _ = (agent, place, kind);
        stale_default_value(Quantity(0))
    }
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
    fn substitute_preferences(&self, agent: EntityId) -> Option<SubstitutePreferences> {
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

pub trait SocialBeliefView {
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        let _ = agent;
        Vec::new()
    }
    fn entity_beliefs_sourced_from_witness(
        &self,
        agent: EntityId,
        witness: EntityId,
    ) -> Vec<(EntityId, BelievedEntityState)> {
        self.agent_belief_store(agent)
            .map(|store| {
                store
                    .known_entities
                    .iter()
                    .filter(|(_, belief)| {
                        matches!(
                            belief.source,
                            PerceptionSource::Report { from, .. } if from == witness
                        )
                    })
                    .map(|(entity, belief)| (*entity, belief.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        let _ = agent;
        None
    }
    fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
        let _ = agent;
        Vec::new()
    }
    fn claim_confidence_threshold(&self, agent: EntityId) -> Permille {
        let _ = agent;
        Permille::ZERO
    }
    fn discrepancy_memory(&self, agent: EntityId) -> Option<&worldwake_core::DiscrepancyMemory> {
        let _ = agent;
        None
    }
    fn blocker_memory(&self, agent: EntityId) -> Option<&worldwake_core::BlockerMemory> {
        let _ = agent;
        None
    }
    fn repair_memory(&self, agent: EntityId) -> Option<&worldwake_core::RepairMemory> {
        let _ = agent;
        None
    }
    fn learned_opportunity_memory(
        &self,
        agent: EntityId,
    ) -> Option<&worldwake_core::LearnedOpportunityMemory> {
        let _ = agent;
        None
    }
    fn survey_memory(&self, agent: EntityId) -> Option<&worldwake_core::SurveyMemory> {
        let _ = agent;
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
}

pub trait PoliticalBeliefView {
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
    fn justice_disposition_profile(&self, agent: EntityId) -> Option<JusticeDispositionProfile> {
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
    fn record_data(&self, record: EntityId) -> Option<RecordData> {
        let _ = record;
        None
    }
    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        let _ = office;
        None
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
    fn visible_reward_encumbrance(
        &self,
        actor: EntityId,
        office: EntityId,
    ) -> Option<&RewardEncumbrance> {
        let _ = (actor, office);
        None
    }
}

pub trait FacilityBeliefView {
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn stock_storage_policy(&self, facility: EntityId) -> Option<StockStoragePolicy> {
        let _ = facility;
        None
    }
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn wash_basin_state(&self, entity: EntityId) -> Option<WashBasinState> {
        let _ = entity;
        None
    }
    fn self_care_occupant(&self, entity: EntityId) -> Option<EntityId> {
        let _ = entity;
        None
    }
    fn rest_site_capacity(&self, place: EntityId) -> Option<NonZeroU32> {
        let _ = place;
        None
    }
    fn rest_site_occupant_count(&self, place: EntityId) -> Option<u32> {
        let _ = place;
        None
    }
    fn is_co_located_with_rest_site(&self, place: EntityId) -> bool {
        let _ = place;
        false
    }
    fn last_harvest_trace(&self, entity: EntityId) -> Option<LastHarvestTrace> {
        let _ = entity;
        None
    }
    fn resource_extraction_queues(&self, entity: EntityId) -> Option<ResourceExtractionQueues> {
        let _ = entity;
        None
    }
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
    + SocialBeliefView
    + PoliticalBeliefView
    + FacilityBeliefView
    + BelievedAuthorityView
    + LocalPhysicalObservationView
{
}

impl<T: SpatialBeliefView + ?Sized> GoalSpatialBeliefView for T {
    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        SpatialBeliefView::effective_place(self, entity)
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        SpatialBeliefView::entities_at(self, place)
    }

    fn believed_entities_at(
        &self,
        agent: EntityId,
        place: EntityId,
        kind: EntityKind,
    ) -> Vec<BeliefValue<EntityId>> {
        SpatialBeliefView::believed_entities_at(self, agent, place, kind)
    }

    fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
        SpatialBeliefView::route_experience(self, agent)
    }

    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        SpatialBeliefView::patrol_route(self, agent)
    }

    fn office_patrol_duty(&self, agent: EntityId) -> Option<OfficePatrolDuty> {
        SpatialBeliefView::office_patrol_duty(self, agent)
    }

    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        SpatialBeliefView::place_has_tag(self, place, tag)
    }

    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        SpatialBeliefView::adjacent_places_with_travel_ticks(self, place)
    }
}

impl<T: TemporalBeliefView + ?Sized> GoalTemporalBeliefView for T {
    fn current_tick(&self) -> Tick {
        TemporalBeliefView::current_tick(self)
    }
}

impl<T: ControlBeliefView + ?Sized> GoalControlBeliefView for T {
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        ControlBeliefView::believed_rights(self, actor, entity)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        ControlBeliefView::can_control(self, actor, entity)
    }
}

impl<T> GoalBeliefView for T
where
    T: GoalSpatialBeliefView
        + GoalTemporalBeliefView
        + GoalControlBeliefView
        + EntityBeliefView
        + ProfileBeliefView
        + InventoryBeliefView
        + CombatBeliefView
        + EconomicBeliefView
        + SocialBeliefView
        + PoliticalBeliefView
        + FacilityBeliefView
        + BelievedAuthorityView
        + LocalPhysicalObservationView
        + ?Sized,
{
    fn current_tick(&self) -> worldwake_core::Tick {
        GoalTemporalBeliefView::current_tick(self)
    }

    fn is_alive(&self, entity: worldwake_core::EntityId) -> bool {
        EntityBeliefView::is_alive(self, entity)
    }

    fn is_dead(&self, entity: worldwake_core::EntityId) -> bool {
        EntityBeliefView::is_dead(self, entity)
    }

    fn locally_observed_is_dead(
        &self,
        agent: worldwake_core::EntityId,
        entity: worldwake_core::EntityId,
    ) -> bool {
        EntityBeliefView::locally_observed_is_dead(self, agent, entity)
    }

    fn entity_kind(&self, entity: worldwake_core::EntityId) -> Option<worldwake_core::EntityKind> {
        EntityBeliefView::entity_kind(self, entity)
    }

    fn effective_place(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        GoalSpatialBeliefView::effective_place(self, entity)
    }

    fn entities_at(&self, place: worldwake_core::EntityId) -> Vec<worldwake_core::EntityId> {
        GoalSpatialBeliefView::entities_at(self, place)
    }

    fn direct_possessions(
        &self,
        holder: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::EntityId> {
        InventoryBeliefView::direct_possessions(self, holder)
    }

    fn known_entity_beliefs(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<(
        worldwake_core::EntityId,
        worldwake_core::BelievedEntityState,
    )> {
        SocialBeliefView::known_entity_beliefs(self, agent)
    }

    fn entity_beliefs_sourced_from_witness(
        &self,
        agent: worldwake_core::EntityId,
        witness: worldwake_core::EntityId,
    ) -> Vec<(
        worldwake_core::EntityId,
        worldwake_core::BelievedEntityState,
    )> {
        SocialBeliefView::entity_beliefs_sourced_from_witness(self, agent, witness)
    }

    fn agent_belief_store(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::AgentBeliefStore> {
        SocialBeliefView::agent_belief_store(self, agent)
    }

    fn known_social_observations(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::SocialObservation> {
        SocialBeliefView::known_social_observations(self, agent)
    }

    fn claim_confidence_threshold(
        &self,
        agent: worldwake_core::EntityId,
    ) -> worldwake_core::Permille {
        SocialBeliefView::claim_confidence_threshold(self, agent)
    }

    fn discrepancy_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::DiscrepancyMemory> {
        SocialBeliefView::discrepancy_memory(self, agent)
    }

    fn blocker_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::BlockerMemory> {
        SocialBeliefView::blocker_memory(self, agent)
    }

    fn repair_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::RepairMemory> {
        SocialBeliefView::repair_memory(self, agent)
    }

    fn learned_opportunity_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::LearnedOpportunityMemory> {
        SocialBeliefView::learned_opportunity_memory(self, agent)
    }

    fn survey_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::SurveyMemory> {
        SocialBeliefView::survey_memory(self, agent)
    }

    fn known_institutional_beliefs(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::BelievedInstitutionalClaim> {
        PoliticalBeliefView::known_institutional_beliefs(self, agent)
    }

    fn actor_lawful_reward_source_for_case(
        &self,
        actor: worldwake_core::EntityId,
        accusation: &worldwake_core::BelievedInstitutionalClaim,
    ) -> Option<worldwake_core::RewardSource> {
        actor_lawful_reward_source_from_beliefs(self, actor, accusation)
    }

    fn visible_reward_encumbrance(
        &self,
        actor: worldwake_core::EntityId,
        office: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::RewardEncumbrance> {
        PoliticalBeliefView::visible_reward_encumbrance(self, actor, office)
    }

    fn factions_of(&self, entity: worldwake_core::EntityId) -> Vec<worldwake_core::EntityId> {
        PoliticalBeliefView::factions_of(self, entity)
    }

    fn bandit_factions_of(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::EntityId> {
        PoliticalBeliefView::bandit_factions_of(self, entity)
    }

    fn locally_observed_bandit_camp_faction_at(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        PoliticalBeliefView::locally_observed_bandit_camp_faction_at(self, agent, place)
    }

    fn believed_activity_of(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<&worldwake_core::BelievedActivity> {
        SocialBeliefView::believed_activity_of(self, entity)
    }

    fn agents_active_at(
        &self,
        place: worldwake_core::EntityId,
        domain: worldwake_core::ActionDomain,
        target: Option<worldwake_core::EntityId>,
    ) -> Vec<worldwake_core::EntityId> {
        SocialBeliefView::agents_active_at(self, place, domain, target)
    }

    fn adjacent_places_with_travel_ticks(
        &self,
        place: worldwake_core::EntityId,
    ) -> Vec<(worldwake_core::EntityId, std::num::NonZeroU32)> {
        GoalSpatialBeliefView::adjacent_places_with_travel_ticks(self, place)
    }

    fn knows_recipe(
        &self,
        actor: worldwake_core::EntityId,
        recipe: worldwake_core::RecipeId,
    ) -> bool {
        InventoryBeliefView::knows_recipe(self, actor, recipe)
    }

    fn known_recipes(&self, agent: worldwake_core::EntityId) -> Vec<worldwake_core::RecipeId> {
        InventoryBeliefView::known_recipes(self, agent)
    }

    fn recipe_definition(&self, recipe: worldwake_core::RecipeId) -> Option<RecipeDefinition> {
        InventoryBeliefView::recipe_definition(self, recipe)
    }

    fn unique_item_count(
        &self,
        holder: worldwake_core::EntityId,
        kind: worldwake_core::UniqueItemKind,
    ) -> u32 {
        InventoryBeliefView::unique_item_count(self, holder, kind)
    }

    fn commodity_quantity(
        &self,
        holder: worldwake_core::EntityId,
        kind: worldwake_core::CommodityKind,
    ) -> worldwake_core::Quantity {
        InventoryBeliefView::commodity_quantity(self, holder, kind)
    }

    fn locally_observed_commodity_quantity(
        &self,
        agent: worldwake_core::EntityId,
        holder: worldwake_core::EntityId,
        kind: worldwake_core::CommodityKind,
    ) -> worldwake_core::Quantity {
        InventoryBeliefView::locally_observed_commodity_quantity(self, agent, holder, kind)
    }

    fn controlled_commodity_quantity_at_place(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        commodity: worldwake_core::CommodityKind,
    ) -> worldwake_core::Quantity {
        EconomicBeliefView::controlled_commodity_quantity_at_place(self, agent, place, commodity)
    }

    fn local_controlled_lots_for(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        commodity: worldwake_core::CommodityKind,
    ) -> Vec<worldwake_core::EntityId> {
        EconomicBeliefView::local_controlled_lots_for(self, agent, place, commodity)
    }

    fn bandit_flee_wound_threshold(
        &self,
        faction: worldwake_core::EntityId,
    ) -> Option<worldwake_core::Permille> {
        EntityBeliefView::bandit_flee_wound_threshold(self, faction)
    }

    fn item_lot_commodity(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::CommodityKind> {
        InventoryBeliefView::item_lot_commodity(self, entity)
    }

    fn item_lot_consumable_profile(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::CommodityConsumableProfile> {
        InventoryBeliefView::item_lot_consumable_profile(self, entity)
    }

    fn direct_container(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        InventoryBeliefView::direct_container(self, entity)
    }

    fn direct_possessor(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        InventoryBeliefView::direct_possessor(self, entity)
    }

    fn believed_rights(
        &self,
        actor: worldwake_core::EntityId,
        entity: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::EffectiveRight> {
        GoalControlBeliefView::believed_rights(self, actor, entity)
    }

    fn workstation_tag(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::WorkstationTag> {
        FacilityBeliefView::workstation_tag(self, entity)
    }

    fn resource_source(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ResourceSource> {
        FacilityBeliefView::resource_source(self, entity)
    }

    fn facility_wash_basin_state(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::WashBasinState> {
        FacilityBeliefView::wash_basin_state(self, entity)
    }

    fn self_care_occupant(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        FacilityBeliefView::self_care_occupant(self, entity)
    }

    fn rest_site_capacity(&self, place: worldwake_core::EntityId) -> Option<std::num::NonZeroU32> {
        FacilityBeliefView::rest_site_capacity(self, place)
    }

    fn rest_site_occupant_count(&self, place: worldwake_core::EntityId) -> Option<u32> {
        FacilityBeliefView::rest_site_occupant_count(self, place)
    }

    fn is_co_located_with_rest_site(&self, place: worldwake_core::EntityId) -> bool {
        FacilityBeliefView::is_co_located_with_rest_site(self, place)
    }

    fn last_harvest_trace(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::LastHarvestTrace> {
        FacilityBeliefView::last_harvest_trace(self, entity)
    }

    fn resource_extraction_queues(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ResourceExtractionQueues> {
        FacilityBeliefView::resource_extraction_queues(self, entity)
    }

    fn resource_sources_at(
        &self,
        place: worldwake_core::EntityId,
        commodity: worldwake_core::CommodityKind,
    ) -> Vec<worldwake_core::EntityId> {
        FacilityBeliefView::resource_sources_at(self, place, commodity)
    }

    fn matching_workstations_at(
        &self,
        place: worldwake_core::EntityId,
        tag: worldwake_core::WorkstationTag,
    ) -> Vec<worldwake_core::EntityId> {
        FacilityBeliefView::matching_workstations_at(self, place, tag)
    }

    fn place_has_tag(
        &self,
        place: worldwake_core::EntityId,
        tag: worldwake_core::PlaceTag,
    ) -> bool {
        GoalSpatialBeliefView::place_has_tag(self, place, tag)
    }

    fn has_production_job(&self, entity: worldwake_core::EntityId) -> bool {
        FacilityBeliefView::has_production_job(self, entity)
    }

    fn can_control(
        &self,
        actor: worldwake_core::EntityId,
        entity: worldwake_core::EntityId,
    ) -> bool {
        GoalControlBeliefView::can_control(self, actor, entity)
    }

    fn stock_storage_policy(
        &self,
        facility: worldwake_core::EntityId,
    ) -> Option<worldwake_core::StockStoragePolicy> {
        FacilityBeliefView::stock_storage_policy(self, facility)
    }

    fn carry_capacity(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::LoadUnits> {
        InventoryBeliefView::carry_capacity(self, entity)
    }

    fn load_of_entity(
        &self,
        entity: worldwake_core::EntityId,
    ) -> Option<worldwake_core::LoadUnits> {
        InventoryBeliefView::load_of_entity(self, entity)
    }

    fn is_incapacitated(&self, entity: worldwake_core::EntityId) -> bool {
        EntityBeliefView::is_incapacitated(self, entity)
    }

    fn has_wounds(&self, entity: worldwake_core::EntityId) -> bool {
        CombatBeliefView::has_wounds(self, entity)
    }

    fn homeostatic_needs(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::HomeostaticNeeds> {
        ProfileBeliefView::homeostatic_needs(self, agent)
    }

    fn drive_thresholds(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::DriveThresholds> {
        ProfileBeliefView::drive_thresholds(self, agent)
    }

    fn metabolism_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::MetabolismProfile> {
        ProfileBeliefView::metabolism_profile(self, agent)
    }

    fn testimony_trust_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::TestimonyTrustProfile> {
        ProfileBeliefView::testimony_trust_profile(self, agent)
    }

    fn route_preference_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::RoutePreferenceProfile> {
        ProfileBeliefView::route_preference_profile(self, agent)
    }

    fn place_sleep_quality_profile(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
    ) -> worldwake_core::SleepQualityProfile {
        ProfileBeliefView::place_sleep_quality_profile(self, agent, place)
    }

    fn place_dirtiness(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
    ) -> worldwake_core::PlaceDirtiness {
        ProfileBeliefView::place_dirtiness(self, agent, place)
    }

    fn latrine_fullness(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
    ) -> worldwake_core::LatrineFullness {
        ProfileBeliefView::latrine_fullness(self, agent, place)
    }

    fn wash_basin_state(
        &self,
        agent: worldwake_core::EntityId,
        basin: worldwake_core::EntityId,
    ) -> worldwake_core::WashBasinState {
        ProfileBeliefView::wash_basin_state(self, agent, basin)
    }

    fn deprivation_exposure(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::DeprivationExposure> {
        ProfileBeliefView::deprivation_exposure(self, agent)
    }

    fn drive_escalation_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::DriveEscalationProfile> {
        ProfileBeliefView::drive_escalation_profile(self, agent)
    }

    fn disposal_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::DisposalProfile> {
        ProfileBeliefView::disposal_profile(self, agent)
    }

    fn exploration_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ExplorationProfile> {
        ProfileBeliefView::exploration_profile(self, agent)
    }

    fn diversification_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::DiversificationProfile> {
        ProfileBeliefView::diversification_profile(self, agent)
    }

    fn last_proactive_exploration_tick(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::Tick> {
        ProfileBeliefView::last_proactive_exploration_tick(self, agent)
    }

    fn acquisition_exhaustion_count(
        &self,
        agent: worldwake_core::EntityId,
        need: worldwake_core::HomeostaticNeedId,
    ) -> u8 {
        ProfileBeliefView::acquisition_exhaustion_count(self, agent, need)
    }

    fn obligation_satiation_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> worldwake_core::ObligationSatiationProfile {
        ProfileBeliefView::obligation_satiation_profile(self, agent)
    }

    fn obligation_execution_tracker(
        &self,
        agent: worldwake_core::EntityId,
    ) -> worldwake_core::ObligationExecutionTracker {
        ProfileBeliefView::obligation_execution_tracker(self, agent)
    }

    fn cognitive_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::CognitiveProfile> {
        ProfileBeliefView::cognitive_profile(self, agent)
    }

    fn portfolio_weights_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> worldwake_core::PortfolioWeightsProfile {
        ProfileBeliefView::portfolio_weights_profile(self, agent)
    }

    fn agent_schema_context_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::AgentSchemaContextProfile> {
        ProfileBeliefView::agent_schema_context_profile(self, agent)
    }

    fn perception_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::PerceptionProfile> {
        ProfileBeliefView::perception_profile(self, agent)
    }

    fn risk_weight_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::RiskWeightProfile> {
        ProfileBeliefView::risk_weight_profile(self, agent)
    }

    fn law_abiding_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::LawAbidingProfile> {
        ProfileBeliefView::law_abiding_profile(self, agent)
    }

    fn belief_confidence_policy(
        &self,
        agent: worldwake_core::EntityId,
    ) -> worldwake_core::BeliefConfidencePolicy {
        SocialBeliefView::belief_confidence_policy(self, agent)
    }

    fn observation_fidelity(&self, agent: worldwake_core::EntityId) -> worldwake_core::Permille {
        SocialBeliefView::observation_fidelity(self, agent)
    }

    fn patrol_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::PatrolProfile> {
        CombatBeliefView::patrol_profile(self, agent)
    }

    fn patrol_route(&self, agent: worldwake_core::EntityId) -> Option<worldwake_core::PatrolRoute> {
        GoalSpatialBeliefView::patrol_route(self, agent)
    }

    fn office_patrol_duty(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::OfficePatrolDuty> {
        GoalSpatialBeliefView::office_patrol_duty(self, agent)
    }

    fn pursuit_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::PursuitProfile> {
        CombatBeliefView::pursuit_profile(self, agent)
    }

    fn epistemic_disposition_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EpistemicDispositionProfile> {
        SocialBeliefView::epistemic_disposition_profile(self, agent)
    }

    fn theft_disposition_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::TheftDispositionProfile> {
        SocialBeliefView::theft_disposition_profile(self, agent)
    }

    fn justice_disposition_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::JusticeDispositionProfile> {
        PoliticalBeliefView::justice_disposition_profile(self, agent)
    }

    fn tell_profile(&self, agent: worldwake_core::EntityId) -> Option<worldwake_core::TellProfile> {
        SocialBeliefView::tell_profile(self, agent)
    }

    fn told_belief_memories(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<(
        worldwake_core::TellMemoryKey,
        worldwake_core::ToldBeliefMemory,
    )> {
        SocialBeliefView::told_belief_memories(self, agent)
    }

    fn told_belief_memory(
        &self,
        actor: worldwake_core::EntityId,
        counterparty: worldwake_core::EntityId,
        topic: &worldwake_core::TellTopic,
    ) -> Option<worldwake_core::ToldBeliefMemory> {
        SocialBeliefView::told_belief_memory(self, actor, counterparty, topic)
    }

    fn recipient_knowledge_status(
        &self,
        actor: worldwake_core::EntityId,
        counterparty: worldwake_core::EntityId,
        topic: &worldwake_core::TellTopic,
    ) -> Option<worldwake_core::RecipientKnowledgeStatus> {
        SocialBeliefView::recipient_knowledge_status(self, actor, counterparty, topic)
    }

    fn ask_witness_memory(
        &self,
        actor: worldwake_core::EntityId,
        key: &worldwake_core::AskWitnessMemoryKey,
    ) -> Option<worldwake_core::AskWitnessMemory> {
        SocialBeliefView::ask_witness_memory(self, actor, key)
    }

    fn courage(&self, agent: worldwake_core::EntityId) -> Option<worldwake_core::Permille> {
        CombatBeliefView::courage(self, agent)
    }

    fn violation_disposition_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ViolationDispositionProfile> {
        PoliticalBeliefView::violation_disposition_profile(self, agent)
    }

    fn active_violation_records(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::RecordedViolation> {
        PoliticalBeliefView::active_violation_records(self, agent)
    }

    fn trade_disposition_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::TradeDispositionProfile> {
        EconomicBeliefView::trade_disposition_profile(self, agent)
    }

    fn merchandise_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::MerchandiseProfile> {
        EconomicBeliefView::merchandise_profile(self, agent)
    }

    fn commodity_valuation_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::CommodityValuationProfile> {
        EconomicBeliefView::commodity_valuation_profile(self, agent)
    }

    fn substitute_preferences(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::SubstitutePreferences> {
        EconomicBeliefView::substitute_preferences(self, agent)
    }

    fn route_experience(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::RouteExperience> {
        GoalSpatialBeliefView::route_experience(self, agent)
    }

    fn source_reliability(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::SourceReliability> {
        SocialBeliefView::source_reliability(self, agent)
    }

    fn preference_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::PreferenceProfile> {
        ProfileBeliefView::preference_profile(self, agent)
    }

    fn expectation_store(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ExpectationStore> {
        SocialBeliefView::expectation_store(self, agent)
    }

    fn last_seen_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::LastSeenMemory> {
        SocialBeliefView::last_seen_memory(self, agent)
    }

    fn utility_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::UtilityProfile> {
        ProfileBeliefView::utility_profile(self, agent)
    }

    fn artifact_posting_profile(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Option<worldwake_core::ArtifactPostingProfile> {
        ProfileBeliefView::artifact_posting_profile(self, agent)
    }

    fn wounds(&self, agent: worldwake_core::EntityId) -> Vec<worldwake_core::Wound> {
        CombatBeliefView::wounds(self, agent)
    }

    fn hostile_targets_of(&self, agent: worldwake_core::EntityId) -> Vec<worldwake_core::EntityId> {
        CombatBeliefView::hostile_targets_of(self, agent)
    }

    fn visible_hostiles_for(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::EntityId> {
        CombatBeliefView::visible_hostiles_for(self, agent)
    }

    fn current_attackers_of(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::EntityId> {
        CombatBeliefView::current_attackers_of(self, agent)
    }

    fn listed_sale_lots_at(
        &self,
        place: worldwake_core::EntityId,
        commodity: worldwake_core::CommodityKind,
    ) -> Vec<worldwake_core::EntityId> {
        EconomicBeliefView::listed_sale_lots_at(self, place, commodity)
    }

    fn seller_for_sale_lot(
        &self,
        lot: worldwake_core::EntityId,
    ) -> Option<worldwake_core::EntityId> {
        EconomicBeliefView::seller_for_sale_lot(self, lot)
    }

    fn has_sale_listing(&self, lot: worldwake_core::EntityId) -> bool {
        EconomicBeliefView::has_sale_listing(self, lot)
    }

    fn demand_memory(
        &self,
        agent: worldwake_core::EntityId,
    ) -> Vec<worldwake_core::DemandObservation> {
        EconomicBeliefView::demand_memory(self, agent)
    }

    fn corpse_entities_at(&self, place: worldwake_core::EntityId) -> Vec<worldwake_core::EntityId> {
        EntityBeliefView::corpse_entities_at(self, place)
    }

    fn record_data(&self, record: worldwake_core::EntityId) -> Option<worldwake_core::RecordData> {
        PoliticalBeliefView::record_data(self, record)
    }

    fn office_data(&self, office: worldwake_core::EntityId) -> Option<worldwake_core::OfficeData> {
        PoliticalBeliefView::office_data(self, office)
    }

    fn believed_force_controller(
        &self,
        office: worldwake_core::EntityId,
    ) -> worldwake_core::InstitutionalBeliefRead<(Option<worldwake_core::EntityId>, bool)> {
        PoliticalBeliefView::believed_force_controller(self, office)
    }

    fn believed_membership(
        &self,
        faction: worldwake_core::EntityId,
        member: worldwake_core::EntityId,
    ) -> worldwake_core::InstitutionalBeliefRead<bool> {
        PoliticalBeliefView::believed_membership(self, faction, member)
    }

    fn believed_faction_rally_point(
        &self,
        faction: worldwake_core::EntityId,
    ) -> worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>> {
        PoliticalBeliefView::believed_faction_rally_point(self, faction)
    }

    fn loyalty_to(
        &self,
        subject: worldwake_core::EntityId,
        target: worldwake_core::EntityId,
    ) -> Option<worldwake_core::Permille> {
        PoliticalBeliefView::loyalty_to(self, subject, target)
    }

    fn believed_support_declaration(
        &self,
        office: worldwake_core::EntityId,
        supporter: worldwake_core::EntityId,
    ) -> worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>> {
        PoliticalBeliefView::believed_support_declaration(self, office, supporter)
    }

    fn believed_support_declarations_for_office(
        &self,
        office: worldwake_core::EntityId,
    ) -> Vec<(
        worldwake_core::EntityId,
        worldwake_core::InstitutionalBeliefRead<Option<worldwake_core::EntityId>>,
    )> {
        PoliticalBeliefView::believed_support_declarations_for_office(self, office)
    }

    fn institutional_belief_claims(
        &self,
        agent: worldwake_core::EntityId,
        key: worldwake_core::InstitutionalBeliefKey,
    ) -> Vec<worldwake_core::BelievedInstitutionalClaim> {
        PoliticalBeliefView::institutional_belief_claims(self, agent, key)
    }

    fn believed_target_location(
        &self,
        agent: worldwake_core::EntityId,
        target: worldwake_core::EntityId,
    ) -> BeliefValue<Option<worldwake_core::EntityId>> {
        EntityBeliefView::believed_target_location(self, agent, target)
    }

    fn believed_entities_at(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        kind: worldwake_core::EntityKind,
    ) -> Vec<BeliefValue<worldwake_core::EntityId>> {
        GoalSpatialBeliefView::believed_entities_at(self, agent, place, kind)
    }

    fn believed_commodity_stock(
        &self,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        kind: worldwake_core::CommodityKind,
    ) -> BeliefValue<worldwake_core::Quantity> {
        InventoryBeliefView::believed_commodity_stock(self, agent, place, kind)
    }
}

fn actor_lawful_reward_source_from_beliefs<V: GoalBeliefView + BelievedAuthorityView + ?Sized>(
    view: &V,
    actor: EntityId,
    accusation: &BelievedInstitutionalClaim,
) -> Option<RewardSource> {
    let (
        worldwake_core::InstitutionalClaim::Accusation { accused, theft, .. },
        worldwake_core::InstitutionalKnowledgeSource::RecordConsultation { record, .. },
    ) = (accusation.claim, accusation.source)
    else {
        return None;
    };
    if theft.quantity == Quantity(0) {
        return None;
    }

    let record_data = view.record_data(record)?;
    if record_data.record_kind != RecordKind::CrimeRegister {
        return None;
    }
    let office = record_data.issuer;
    let office_data = view.office_data(office)?;
    if view.effective_place(actor) != Some(office_data.seat) {
        return None;
    }
    if !matches!(
        view.believed_office_holder(office),
        BeliefRead::Known(holder) | BeliefRead::Stale(holder) if holder.value == Some(actor)
    ) {
        return None;
    }
    if !view
        .believed_rights(actor, accused)
        .iter()
        .any(|right| right.kind == RightKind::JurisdictionalAuthority && right.via == Some(office))
    {
        return None;
    }

    let reward_commodity = CommodityKind::Coin;
    let local_balance =
        view.controlled_commodity_quantity_at_place(actor, office_data.seat, reward_commodity);
    let reserved = view
        .visible_reward_encumbrance(actor, office)
        .map_or(Quantity(0), |encumbrance| {
            encumbrance.reserved_quantity(reward_commodity)
        });
    if local_balance.0.saturating_sub(reserved.0) == 0 {
        return None;
    }

    Some(RewardSource::InstitutionalTreasury {
        treasury_entity: office,
    })
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
        DurationExpr::Variable { max, .. } => Some(ActionDuration::new(max.get())),
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
                crate::MetabolismDurationKind::CleanBasin => {
                    profile.clean_basin_duration_ticks.get()
                }
                crate::MetabolismDurationKind::EmptyLatrine => {
                    profile.empty_latrine_duration_ticks.get()
                }
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
    use super::{
        BeliefRead, BeliefStatus, BeliefValue, BelievedAuthorityView, LocalPhysicalObservationView,
        ObservationSource, ObservedRead, estimate_duration_from_beliefs,
    };
    use crate::{
        ActionPayload, CombatBeliefView, DurationExpr, EconomicBeliefView, EntityBeliefView,
        FacilityBeliefView, GoalBeliefView, GoalControlBeliefView, GoalSpatialBeliefView,
        GoalTemporalBeliefView, InventoryBeliefView, PerAgentBeliefView, ProfileBeliefView,
        SocialBeliefView,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState, BelievedInstitutionalClaim,
        CauseRef, ClaimId, ClaimValue, CommodityConsumableProfile, CommodityKind, Container,
        ControlSource, DemandObservation, DeprivationExposure, DiversificationProfile,
        DriveEscalationProfile, DriveThresholds, EntityBeliefAspect, EntityBeliefClaim, EntityId,
        EntityKind, EventLog, GroundComfortTag, HomeostaticNeedId, HomeostaticNeeds,
        InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource,
        LastProactiveExplorationTick, LatrineFullness, LoadUnits, ObservationOmissionLog,
        OfficeData, PatrolProfile, PerceptionProfile, PerceptionSource, Permille, PlaceDirtiness,
        PortfolioWeightsProfile, Quantity, RecordData, RecordEntryId, RecordKind, ResourceSource,
        RewardEncumbrance, RewardReservation, RoutePreferenceProfile, ShelterTag,
        SleepQualityProfile, SleepRecoveryModifier, SuccessionLaw, SurveyMemory,
        TestimonyTrustProfile, TheftFacts, Tick, UniqueItemKind, ViolationId, VisibilitySpec,
        WashBasinState, WitnessData, WorkstationTag, World, WorldTxn, build_prototype_world,
    };

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

    struct StubGoalBeliefView;

    impl LocalPhysicalObservationView for StubGoalBeliefView {}

    impl GoalSpatialBeliefView for StubGoalBeliefView {
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, std::num::NonZeroU32)> {
            Vec::new()
        }
    }

    impl GoalControlBeliefView for StubGoalBeliefView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
    }

    impl GoalTemporalBeliefView for StubGoalBeliefView {}

    impl EntityBeliefView for StubGoalBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for StubGoalBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn deprivation_exposure(&self, _agent: EntityId) -> Option<DeprivationExposure> {
            None
        }

        fn drive_escalation_profile(&self, _agent: EntityId) -> Option<DriveEscalationProfile> {
            None
        }

        fn metabolism_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MetabolismProfile> {
            None
        }

        fn testimony_trust_profile(&self, _agent: EntityId) -> Option<TestimonyTrustProfile> {
            None
        }

        fn route_preference_profile(&self, _agent: EntityId) -> Option<RoutePreferenceProfile> {
            None
        }

        fn exploration_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::ExplorationProfile> {
            None
        }

        fn diversification_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::DiversificationProfile> {
            None
        }

        fn last_proactive_exploration_tick(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::Tick> {
            None
        }

        fn acquisition_exhaustion_count(&self, _agent: EntityId, _need: HomeostaticNeedId) -> u8 {
            0
        }
    }

    impl InventoryBeliefView for StubGoalBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
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

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }
    }

    impl CombatBeliefView for StubGoalBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EconomicBeliefView for StubGoalBeliefView {
        fn trade_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TradeDispositionProfile> {
            None
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

        fn merchandise_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MerchandiseProfile> {
            None
        }
    }

    impl SocialBeliefView for StubGoalBeliefView {
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
    }

    impl crate::PoliticalBeliefView for StubGoalBeliefView {}

    impl BelievedAuthorityView for StubGoalBeliefView {}

    impl FacilityBeliefView for StubGoalBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
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

    fn assert_goal_hygiene_reads<V: GoalBeliefView>(
        view: &V,
        actor: EntityId,
        place: EntityId,
        basin: EntityId,
        dirtiness: PlaceDirtiness,
        fullness: LatrineFullness,
        basin_state: WashBasinState,
    ) {
        assert_eq!(view.place_dirtiness(actor, place), dirtiness);
        assert_eq!(view.latrine_fullness(actor, place), fullness);
        assert_eq!(view.wash_basin_state(actor, basin), basin_state);
    }

    fn observed_entity(
        place: EntityId,
        kind: EntityKind,
        commodity: CommodityKind,
    ) -> BelievedEntityState {
        let mut state = BelievedEntityState::single_observation_defaults(
            Tick(1),
            PerceptionSource::DirectObservation,
        );
        state.believed_kind = Some(kind);
        state.last_known_place = Some(place);
        state.last_known_inventory.insert(commodity, Quantity(1));
        state
    }

    #[test]
    fn entity_beliefs_sourced_from_witness_filters_report_provenance_in_key_order() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor;
        let witness;
        let other_witness;
        let subject_low = EntityId {
            slot: 20,
            generation: 0,
        };
        let subject_high = EntityId {
            slot: 30,
            generation: 0,
        };
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            witness = txn.create_agent("Bryn", ControlSource::Ai).unwrap();
            other_witness = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(witness, place).unwrap();
            txn.set_ground_location(other_witness, place).unwrap();

            let mut beliefs = AgentBeliefStore::new();
            let mut from_witness_high =
                observed_entity(place, EntityKind::Agent, CommodityKind::Bread);
            from_witness_high.source = PerceptionSource::Report {
                from: witness,
                chain_len: 1,
            };
            let mut direct = observed_entity(place, EntityKind::Agent, CommodityKind::Apple);
            direct.source = PerceptionSource::DirectObservation;
            let mut from_other_witness =
                observed_entity(place, EntityKind::Agent, CommodityKind::Grain);
            from_other_witness.source = PerceptionSource::Report {
                from: other_witness,
                chain_len: 1,
            };
            let mut from_witness_low =
                observed_entity(place, EntityKind::Agent, CommodityKind::Coin);
            from_witness_low.source = PerceptionSource::Report {
                from: witness,
                chain_len: 1,
            };
            beliefs
                .known_entities
                .insert(subject_high, from_witness_high);
            beliefs.known_entities.insert(
                EntityId {
                    slot: 10,
                    generation: 0,
                },
                direct,
            );
            beliefs.known_entities.insert(
                EntityId {
                    slot: 40,
                    generation: 0,
                },
                from_other_witness,
            );
            beliefs.known_entities.insert(subject_low, from_witness_low);
            txn.set_component_agent_belief_store(actor, beliefs)
                .unwrap();
            commit_txn(txn);
        }
        let view = PerAgentBeliefView::from_world(actor, &world);

        let entries = SocialBeliefView::entity_beliefs_sourced_from_witness(&view, actor, witness);

        assert_eq!(
            entries
                .iter()
                .map(|(entity, _)| *entity)
                .collect::<Vec<_>>(),
            vec![subject_low, subject_high]
        );
        assert!(entries.iter().all(|(_, belief)| matches!(
            belief.source,
            PerceptionSource::Report { from, .. } if from == witness
        )));
    }

    fn accusation_case(
        accuser: EntityId,
        suspect: EntityId,
        missing_entity: EntityId,
        expected_place: EntityId,
        record: EntityId,
        theft_commodity: CommodityKind,
    ) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::Accusation {
                accuser,
                accused: suspect,
                violation_id: ViolationId(1),
                theft: TheftFacts {
                    missing_entity,
                    expected_place,
                    commodity: theft_commodity,
                    quantity: Quantity(4),
                },
                effective_tick: Tick(1),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record,
                entry_id: RecordEntryId(1),
            },
            learned_tick: Tick(1),
            learned_at: Some(expected_place),
        }
    }

    fn create_crime_register(txn: &mut WorldTxn<'_>, seat: EntityId, office: EntityId) -> EntityId {
        txn.create_record(RecordData {
            record_kind: RecordKind::CrimeRegister,
            home_place: seat,
            issuer: office,
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: Vec::new(),
            next_entry_id: 0,
        })
        .unwrap()
    }

    struct RewardSourceFixture {
        world: World,
        actor: EntityId,
        accusation: BelievedInstitutionalClaim,
    }

    fn reward_source_fixture(
        actor_holds_office: bool,
        funds: Quantity,
        reserved: Quantity,
    ) -> RewardSourceFixture {
        reward_source_fixture_with_commodities(
            actor_holds_office,
            funds,
            CommodityKind::Coin,
            reserved,
            CommodityKind::Coin,
        )
    }

    fn reward_source_fixture_with_commodities(
        actor_holds_office: bool,
        funds: Quantity,
        treasury_commodity: CommodityKind,
        reserved: Quantity,
        theft_commodity: CommodityKind,
    ) -> RewardSourceFixture {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let seat = places[0];
        let jurisdiction_place = *places.get(1).unwrap_or(&seat);
        let (actor, _office, accused, missing_entity, record) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = if actor_holds_office {
                actor
            } else {
                txn.create_agent("Bram", ControlSource::Ai).unwrap()
            };
            let accused = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, seat).unwrap();
            txn.set_ground_location(holder, seat).unwrap();
            txn.set_ground_location(accused, jurisdiction_place)
                .unwrap();

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
            txn.create_record(RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: seat,
                issuer: office,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
            txn.assign_office(office, holder).unwrap();
            let record = create_crime_register(&mut txn, seat, office);
            let missing_entity = txn.create_item_lot(theft_commodity, Quantity(4)).unwrap();
            txn.set_ground_location(missing_entity, jurisdiction_place)
                .unwrap();

            let maybe_lot = if funds == Quantity(0) {
                None
            } else {
                let container = txn
                    .create_container(Container {
                        capacity: LoadUnits(100),
                        allowed_commodities: None,
                        allows_unique_items: false,
                        allows_nested_containers: false,
                    })
                    .unwrap();
                txn.set_ground_location(container, seat).unwrap();
                txn.set_owner(container, office).unwrap();
                let lot = txn
                    .create_item_lot_with_owner(treasury_commodity, funds, seat, Some(office))
                    .unwrap();
                txn.put_into_container(lot, container).unwrap();
                Some(lot)
            };
            if reserved > Quantity(0) {
                txn.set_component_reward_encumbrance(
                    office,
                    RewardEncumbrance::from_reservation(RewardReservation {
                        bounty_artifact: record,
                        commodity: CommodityKind::Coin,
                        quantity: reserved,
                    }),
                )
                .unwrap();
            }

            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                accused,
                observed_entity(jurisdiction_place, EntityKind::Agent, theft_commodity),
            );
            if let Some(lot) = maybe_lot {
                beliefs.update_entity(
                    lot,
                    observed_entity(seat, EntityKind::ItemLot, treasury_commodity),
                );
            }
            beliefs.record_institutional_belief(
                InstitutionalBeliefKey::OfficeHolderOf { office },
                BelievedInstitutionalClaim {
                    claim: InstitutionalClaim::OfficeHolder {
                        office,
                        holder: Some(holder),
                        effective_tick: Tick(1),
                    },
                    source: InstitutionalKnowledgeSource::WitnessedEvent,
                    learned_tick: Tick(1),
                    learned_at: Some(seat),
                },
                &PerceptionProfile::default(),
            );
            txn.set_component_agent_belief_store(actor, beliefs)
                .unwrap();
            commit_txn(txn);
            (actor, office, accused, missing_entity, record)
        };
        let accusation = accusation_case(
            actor,
            accused,
            missing_entity,
            jurisdiction_place,
            record,
            theft_commodity,
        );

        RewardSourceFixture {
            world,
            actor,
            accusation,
        }
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

        assert_eq!(GoalBeliefView::expectation_store(&view, agent), None);
        assert_eq!(GoalBeliefView::last_seen_memory(&view, agent), None);
    }

    #[test]
    fn goal_belief_view_observation_omission_log_reads_agent_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::observation_omission_log(&view, actor),
            Some(&ObservationOmissionLog::default())
        );
    }

    #[test]
    fn goal_belief_view_acquisition_exhaustion_count_defaults_to_zero() {
        let view = StubGoalBeliefView;
        let agent = EntityId {
            slot: 1,
            generation: 0,
        };

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
    }

    #[test]
    fn goal_belief_view_place_sleep_quality_profile_is_belief_scoped() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let home = places[0];
        let known_place = places[1];
        let unknown_place = places[2];
        let known_profile = SleepQualityProfile {
            shelter: ShelterTag::Shelter,
            ground_comfort: GroundComfortTag::Soft,
            recovery_modifier: SleepRecoveryModifier::new(1250),
        };
        let unknown_profile = SleepQualityProfile {
            shelter: ShelterTag::Roofed,
            ground_comfort: GroundComfortTag::Hard,
            recovery_modifier: SleepRecoveryModifier::new(700),
        };
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, home).unwrap();
            txn.set_component_sleep_quality_profile(known_place, known_profile)
                .unwrap();
            txn.set_component_sleep_quality_profile(unknown_place, unknown_profile)
                .unwrap();

            let mut beliefs = AgentBeliefStore::default();
            let mut known_state = BelievedEntityState::single_observation_defaults(
                Tick(1),
                PerceptionSource::DirectObservation,
            );
            known_state.believed_kind = Some(EntityKind::Place);
            known_state.last_known_place = Some(known_place);
            beliefs.update_entity(known_place, known_state);
            txn.set_component_agent_belief_store(actor, beliefs)
                .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::place_sleep_quality_profile(&view, actor, known_place),
            known_profile
        );
        assert_eq!(
            GoalBeliefView::place_sleep_quality_profile(&view, actor, unknown_place),
            SleepQualityProfile::default()
        );
    }

    #[test]
    fn place_dirtiness_accessor_returns_authoritative_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let dirtiness = PlaceDirtiness {
            value: Permille::new(500).unwrap(),
            decay_per_tick: Permille::new(3).unwrap(),
            dirtiness_per_use: Permille::new(90).unwrap(),
        };
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_place_dirtiness(place, dirtiness).unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::place_dirtiness(&view, actor, place),
            dirtiness
        );
    }

    #[test]
    fn place_hygiene_accessors_do_not_reveal_known_remote_dynamic_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let home = places[0];
        let remote = places[1];
        let remote_dirtiness = PlaceDirtiness {
            value: Permille::new(700).unwrap(),
            ..PlaceDirtiness::default()
        };
        let remote_fullness = LatrineFullness {
            fill: Permille::new(900).unwrap(),
            ..LatrineFullness::default()
        };
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, home).unwrap();
            txn.set_component_place_dirtiness(remote, remote_dirtiness)
                .unwrap();
            txn.set_component_latrine_fullness(remote, remote_fullness)
                .unwrap();

            let mut beliefs = AgentBeliefStore::default();
            let mut remote_state = BelievedEntityState::single_observation_defaults(
                Tick(1),
                PerceptionSource::DirectObservation,
            );
            remote_state.believed_kind = Some(EntityKind::Place);
            remote_state.last_known_place = Some(remote);
            beliefs.update_entity(remote, remote_state);
            txn.set_component_agent_belief_store(actor, beliefs)
                .unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::place_dirtiness(&view, actor, remote),
            PlaceDirtiness::default()
        );
        assert_eq!(
            GoalBeliefView::latrine_fullness(&view, actor, remote),
            LatrineFullness::default()
        );
    }

    #[test]
    fn latrine_fullness_accessor_returns_default_for_unauthored_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            commit_txn(txn);
            actor
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::latrine_fullness(&view, actor, place),
            LatrineFullness::default()
        );
    }

    #[test]
    fn wash_basin_state_accessor_returns_default_for_non_basin_facility() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, facility) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(facility, place).unwrap();
            commit_txn(txn);
            (actor, facility)
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::wash_basin_state(&view, actor, facility),
            WashBasinState::default()
        );
    }

    #[test]
    fn goal_belief_view_forwards_hygiene_accessors() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let dirtiness = PlaceDirtiness {
            value: Permille::new(420).unwrap(),
            ..PlaceDirtiness::default()
        };
        let fullness = LatrineFullness {
            fill: Permille::new(640).unwrap(),
            ..LatrineFullness::default()
        };
        let basin_state = WashBasinState {
            clean_water_units: 4,
            max_clean_water: 12,
            ..WashBasinState::default()
        };
        let (actor, basin) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_place_dirtiness(place, dirtiness).unwrap();
            txn.set_component_latrine_fullness(place, fullness).unwrap();
            let basin = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(basin, place).unwrap();
            txn.set_component_wash_basin_state(basin, basin_state)
                .unwrap();
            commit_txn(txn);
            (actor, basin)
        };
        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_goal_hygiene_reads(&view, actor, place, basin, dirtiness, fullness, basin_state);
    }

    #[test]
    fn goal_belief_view_diversification_defaults_return_none() {
        let view = StubGoalBeliefView;
        let agent = EntityId {
            slot: 1,
            generation: 0,
        };

        assert_eq!(GoalBeliefView::diversification_profile(&view, agent), None);
        assert_eq!(
            GoalBeliefView::last_proactive_exploration_tick(&view, agent),
            None
        );
    }

    #[test]
    fn goal_belief_view_memory_accessors_default_to_none() {
        let view = StubGoalBeliefView;
        let agent = EntityId {
            slot: 1,
            generation: 0,
        };

        assert_eq!(GoalBeliefView::discrepancy_memory(&view, agent), None);
        assert_eq!(GoalBeliefView::blocker_memory(&view, agent), None);
        assert_eq!(GoalBeliefView::repair_memory(&view, agent), None);
        assert_eq!(
            GoalBeliefView::learned_opportunity_memory(&view, agent),
            None
        );
        assert_eq!(GoalBeliefView::survey_memory(&view, agent), None);
    }

    #[test]
    fn runtime_belief_view_survey_memory_returns_component() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };
        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(
            GoalBeliefView::survey_memory(&view, agent),
            Some(&SurveyMemory::default())
        );
    }

    #[test]
    fn runtime_belief_view_s151_profile_accessors_return_seeded_defaults() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };
        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(
            GoalBeliefView::testimony_trust_profile(&view, agent),
            Some(TestimonyTrustProfile::default())
        );
        assert_eq!(
            GoalBeliefView::route_preference_profile(&view, agent),
            Some(RoutePreferenceProfile::default())
        );
    }

    #[test]
    fn goal_belief_view_portfolio_weights_profile_returns_world_component() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let custom = PortfolioWeightsProfile {
            need_survival: Permille::new(910).unwrap(),
            pain_care: Permille::new(820).unwrap(),
            obligation_duty: Permille::new(730).unwrap(),
            economic_opportunity: Permille::new(640).unwrap(),
            social_motive: Permille::new(550).unwrap(),
            max_plans_normal: 6,
            max_plans_emergency: 4,
            max_plans_idle: 7,
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_component_portfolio_weights_profile(agent, custom)
                .unwrap();
            commit_txn(txn);
            agent
        };
        let view = PerAgentBeliefView::from_world(agent, &world);

        assert_eq!(
            GoalBeliefView::portfolio_weights_profile(&view, agent),
            custom
        );
    }

    #[test]
    fn accessor_hides_institutional_source_without_believed_record_snapshot() {
        let fixture = reward_source_fixture(true, Quantity(5), Quantity(1));
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn accessor_hides_non_coin_institutional_source_without_believed_record_snapshot() {
        let fixture = reward_source_fixture_with_commodities(
            true,
            Quantity(5),
            CommodityKind::Coin,
            Quantity(1),
            CommodityKind::Bread,
        );
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn accessor_returns_none_for_non_holder() {
        let fixture = reward_source_fixture(false, Quantity(5), Quantity(0));
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn accessor_returns_none_when_office_has_no_treasury() {
        let fixture = reward_source_fixture(true, Quantity(0), Quantity(0));
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn accessor_returns_none_when_office_funds_are_fully_encumbered() {
        let fixture = reward_source_fixture(true, Quantity(5), Quantity(5));
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn accessor_returns_none_for_non_coin_theft_with_only_non_coin_treasury() {
        let fixture = reward_source_fixture_with_commodities(
            true,
            Quantity(5),
            CommodityKind::Bread,
            Quantity(0),
            CommodityKind::Bread,
        );
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);

        assert_eq!(
            GoalBeliefView::actor_lawful_reward_source_for_case(
                &view,
                fixture.actor,
                &fixture.accusation
            ),
            None
        );
    }

    #[test]
    fn per_agent_goal_belief_view_exposes_diversification_components() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_agent_belief_store(actor, AgentBeliefStore::default())
                .unwrap();
            txn.set_component_diversification_profile(
                actor,
                DiversificationProfile {
                    base_curiosity: Permille::new(610).unwrap(),
                    comfort_threshold: Permille::new(320).unwrap(),
                    curiosity_buildup_rate: Permille::new(9).unwrap(),
                    exploration_cooldown_ticks: 19,
                    familiarity_per_visit: Permille::new(160).unwrap(),
                    familiarity_recovery_per_tick: Permille::new(4).unwrap(),
                    familiarity_floor: Permille::new(70).unwrap(),
                    max_exploration_hops: 7,
                },
            )
            .unwrap();
            txn.set_component_last_proactive_exploration_tick(
                actor,
                LastProactiveExplorationTick(Some(Tick(42))),
            )
            .unwrap();
            commit_txn(txn);
            actor
        };

        let view = PerAgentBeliefView::from_world(actor, &world);

        assert_eq!(
            GoalBeliefView::diversification_profile(&view, actor),
            Some(DiversificationProfile {
                base_curiosity: Permille::new(610).unwrap(),
                comfort_threshold: Permille::new(320).unwrap(),
                curiosity_buildup_rate: Permille::new(9).unwrap(),
                exploration_cooldown_ticks: 19,
                familiarity_per_visit: Permille::new(160).unwrap(),
                familiarity_recovery_per_tick: Permille::new(4).unwrap(),
                familiarity_floor: Permille::new(70).unwrap(),
                max_exploration_hops: 7,
            })
        );
        assert_eq!(
            GoalBeliefView::last_proactive_exploration_tick(&view, actor),
            Some(Tick(42))
        );
    }

    #[test]
    fn belief_status_for_effective_confidence_uses_threshold_bands() {
        let threshold = Permille::new(300).unwrap();

        assert_eq!(
            super::belief_status_for_effective_confidence(650, threshold),
            super::BeliefStatus::Certain
        );
        assert_eq!(
            super::belief_status_for_effective_confidence(300, threshold),
            super::BeliefStatus::Probable
        );
        assert_eq!(
            super::belief_status_for_effective_confidence(299, threshold),
            super::BeliefStatus::Stale
        );
    }

    #[test]
    fn project_claim_into_belief_value_marks_refuted_claims_contradicted() {
        let subject = EntityId {
            slot: 40,
            generation: 0,
        };
        let value = super::project_claim_into_belief_value(
            &EntityBeliefClaim {
                refuted_at_tick: Some(Tick(9)),
                ..sample_claim(
                    1,
                    subject,
                    EntityBeliefAspect::Location,
                    ClaimValue::Place(Some(EntityId {
                        slot: 10,
                        generation: 0,
                    })),
                    7,
                    950,
                )
            },
            Some(EntityId {
                slot: 10,
                generation: 0,
            }),
            Tick(10),
            Permille::new(300).unwrap(),
            &BeliefConfidencePolicy::default(),
        );

        assert_eq!(value.status, super::BeliefStatus::Contradicted);
        assert_eq!(value.claimed_event_tick, Some(Tick(7)));
    }

    #[test]
    fn project_claims_into_belief_set_marks_disputed_when_alternative_values_survive() {
        let subject = EntityId {
            slot: 41,
            generation: 0,
        };
        let set = super::project_claims_into_belief_set(
            [
                (
                    sample_claim(
                        1,
                        subject,
                        EntityBeliefAspect::Location,
                        ClaimValue::Place(Some(EntityId {
                            slot: 10,
                            generation: 0,
                        })),
                        7,
                        950,
                    ),
                    Some(EntityId {
                        slot: 10,
                        generation: 0,
                    }),
                ),
                (
                    sample_claim(
                        2,
                        subject,
                        EntityBeliefAspect::Location,
                        ClaimValue::Place(Some(EntityId {
                            slot: 11,
                            generation: 0,
                        })),
                        9,
                        975,
                    ),
                    Some(EntityId {
                        slot: 11,
                        generation: 0,
                    }),
                ),
            ],
            Tick(10),
            Permille::new(300).unwrap(),
            &BeliefConfidencePolicy::default(),
        );

        assert_eq!(
            set.best.as_ref().map(|best| best.value),
            Some(Some(EntityId {
                slot: 11,
                generation: 0,
            }))
        );
        assert_eq!(
            set.best.as_ref().map(|best| best.status),
            Some(super::BeliefStatus::Disputed)
        );
        assert_eq!(set.alternatives.len(), 1);
        assert_eq!(
            set.alternatives[0].value,
            Some(EntityId {
                slot: 10,
                generation: 0,
            })
        );
    }

    #[test]
    fn project_claims_into_belief_set_deduplicates_same_value_alternatives() {
        let subject = EntityId {
            slot: 42,
            generation: 0,
        };
        let set = super::project_claims_into_belief_set(
            [
                (
                    sample_claim(
                        1,
                        subject,
                        EntityBeliefAspect::Inventory(CommodityKind::Bread),
                        ClaimValue::Quantity(Quantity(3)),
                        8,
                        960,
                    ),
                    Quantity(3),
                ),
                (
                    sample_claim(
                        2,
                        subject,
                        EntityBeliefAspect::Inventory(CommodityKind::Bread),
                        ClaimValue::Quantity(Quantity(3)),
                        9,
                        980,
                    ),
                    Quantity(3),
                ),
            ],
            Tick(10),
            Permille::new(300).unwrap(),
            &BeliefConfidencePolicy::default(),
        );

        assert_eq!(set.best.as_ref().map(|best| best.value), Some(Quantity(3)));
        assert_eq!(
            set.best.as_ref().map(|best| best.status),
            Some(super::BeliefStatus::Certain)
        );
        assert!(set.alternatives.is_empty());
    }

    #[test]
    fn project_claims_into_belief_set_prefers_active_claims_over_contradicted_history() {
        let subject = EntityId {
            slot: 43,
            generation: 0,
        };
        let set = super::project_claims_into_belief_set(
            [
                (
                    EntityBeliefClaim {
                        refuted_at_tick: Some(Tick(9)),
                        ..sample_claim(
                            1,
                            subject,
                            EntityBeliefAspect::Location,
                            ClaimValue::Place(Some(EntityId {
                                slot: 10,
                                generation: 0,
                            })),
                            7,
                            950,
                        )
                    },
                    Some(EntityId {
                        slot: 10,
                        generation: 0,
                    }),
                ),
                (
                    sample_claim(
                        2,
                        subject,
                        EntityBeliefAspect::Location,
                        ClaimValue::Place(Some(EntityId {
                            slot: 11,
                            generation: 0,
                        })),
                        10,
                        920,
                    ),
                    Some(EntityId {
                        slot: 11,
                        generation: 0,
                    }),
                ),
            ],
            Tick(10),
            Permille::new(300).unwrap(),
            &BeliefConfidencePolicy::default(),
        );

        assert_eq!(
            set.best.as_ref().map(|best| best.value),
            Some(Some(EntityId {
                slot: 11,
                generation: 0,
            }))
        );
        assert_eq!(
            set.best.as_ref().map(|best| best.status),
            Some(super::BeliefStatus::Certain)
        );
        assert!(set.alternatives.is_empty());
    }

    #[test]
    fn belief_read_encodes_unknown_known_and_stale() {
        let value = BeliefValue {
            value: EntityId {
                slot: 7,
                generation: 0,
            },
            confidence: Permille::new(900).unwrap(),
            acquired_tick: Tick(12),
            claimed_event_tick: Some(Tick(11)),
            status: BeliefStatus::Probable,
        };

        assert!(matches!(
            BeliefRead::<EntityId>::Unknown,
            BeliefRead::Unknown
        ));
        match BeliefRead::Known(value) {
            BeliefRead::Known(known) => {
                assert_eq!(known.value.slot, 7);
                assert_eq!(known.confidence, Permille::new(900).unwrap());
                assert_eq!(known.acquired_tick, Tick(12));
            }
            BeliefRead::Unknown | BeliefRead::Stale(_) => panic!("expected known belief read"),
        }
        match BeliefRead::Stale(value) {
            BeliefRead::Stale(stale) => {
                assert_eq!(stale.value.slot, 7);
                assert_eq!(stale.status, BeliefStatus::Probable);
            }
            BeliefRead::Unknown | BeliefRead::Known(_) => panic!("expected stale belief read"),
        }
    }

    #[test]
    fn observed_read_carries_tick_and_source() {
        let observed = ObservedRead {
            value: Quantity(4),
            observed_tick: Tick(21),
            source: ObservationSource::CoLocatedSameTick,
        };

        assert_eq!(observed.value, Quantity(4));
        assert_eq!(observed.observed_tick, Tick(21));
        assert_eq!(observed.source, ObservationSource::CoLocatedSameTick);
        assert_ne!(
            ObservationSource::CoLocatedSameTick,
            ObservationSource::BeliefStoreSnapshot
        );
    }

    #[test]
    fn local_physical_observation_view_defaults_return_empty_same_tick_reads() {
        struct EmptyObservationView;
        impl LocalPhysicalObservationView for EmptyObservationView {}

        let view = EmptyObservationView;
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let subject = EntityId {
            slot: 2,
            generation: 0,
        };

        let colocated = view.colocated_entities(actor);
        assert!(colocated.value.is_empty());
        assert_eq!(colocated.observed_tick, Tick(0));
        assert_eq!(colocated.source, ObservationSource::CoLocatedSameTick);
        assert_eq!(view.observed_item_lot_quantity(subject).value, None);
        assert_eq!(view.observed_workstation_tag(subject).value, None);
        assert_eq!(view.observed_resource_source(subject).value, None);
        assert!(view.observed_container_contents(subject).value.is_empty());
        assert_eq!(view.observed_entity_kind(subject).value, None);
    }

    #[test]
    fn believed_authority_view_defaults_return_unknown() {
        struct EmptyAuthorityView;
        impl BelievedAuthorityView for EmptyAuthorityView {}

        let view = EmptyAuthorityView;
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let subject = EntityId {
            slot: 2,
            generation: 0,
        };

        assert!(matches!(
            view.believed_owner_of(subject),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            view.believed_holder_of(subject),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            view.believed_access_right(actor, subject),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            view.believed_jurisdiction(subject),
            BeliefRead::Unknown
        ));
        assert!(matches!(
            view.believed_office_holder(subject),
            BeliefRead::Unknown
        ));
    }
}
