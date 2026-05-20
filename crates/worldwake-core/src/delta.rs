//! Typed event-log deltas over canonical world semantics.

use crate::BeliefStoreDiff;
use crate::LatrineFullness;
use crate::{
    AcquisitionExhaustionTracker, AgendaProfile, AgentBeliefStore, AgentData,
    AgentSchemaContextProfile, ArtifactHeader, ArtifactPostingProfile, BanditCamp,
    BanditFactionPolicy, BlockerMemory, BountyTerms, CarryCapacity, CognitiveArchetypeComponent,
    CognitiveProfile, CombatProfile, CombatStance, CommodityKind, CommodityValuationProfile,
    CommunicationProfile, Container, ContentionDispositionProfile, ContentionIntents,
    ContentionPolicy, ContentionQueue, DeadAt, DemandMemory, DeprivationExposure,
    DiscrepancyMemory, DisposalProfile, DiversificationProfile, DriveEscalationProfile,
    DriveThresholds, EntityId, EntityKind, EpistemicDispositionProfile, ExecutionBudget,
    ExpectationStore, ExplorationProfile, FactionData, GroundSince, HomeostaticNeeds,
    InTransitOnEdge, IntentionDispositionProfile, IntentionFrame, ItemLot,
    JusticeDispositionProfile, KnownRecipes, LastHarvestTrace, LastProactiveExplorationTick,
    LastSeenMemory, LawAbidingProfile, LearnedOpportunityMemory, MemoryCapacityProfile,
    MerchandiseProfile, MetabolismProfile, Name, NoticeContent, ObligationExecutionTracker,
    ObligationSatiationProfile, OfficeData, OfficeForceProfile, OfficeForceState, OfficePatrolDuty,
    PatrolProfile, PatrolRoute, PerceptionProfile, Permille, PlaceDirtiness,
    PlaceVisibilityProfile, PortfolioWeightsProfile, PreferenceProfile, ProductionJob,
    ProductionOutputOwnershipPolicy, PursuitProfile, Quantity, RecordData, RepairMemory,
    ReservationRecord, ResourceExtractionQueues, ResourceSource, RewardEncumbrance,
    RiskWeightProfile, RouteExperience, RoutePreferenceProfile, SaleListing, SceneEvidence,
    SleepEpisode, SleepQualityProfile, SourceReliability, StockAssignment, StockStoragePolicy,
    SubstitutePreferences, SurveyMemory, TellProfile, TestimonyTrustProfile,
    TheftDispositionProfile, TradeDispositionProfile, UniqueItem, UtilityProfile,
    ViolationDispositionProfile, ViolationMemory, WashBasinState, WorkstationMarker, WoundList,
    component_schema::with_component_schema_entries,
};
use serde::{Deserialize, Serialize};

macro_rules! define_component_kind {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr_2021, $component_variant:ident })*) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        pub enum ComponentKind {
            $($component_variant,)*
        }

        impl ComponentKind {
            pub const ALL: [Self; with_component_schema_entries!(forward_authoritative_components, count_authoritative_components)] = [
                $(Self::$component_variant,)*
            ];
        }
    };
}

macro_rules! define_component_value {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr_2021, $component_variant:ident })*) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum ComponentValue {
            $($component_variant($component_ty),)*
        }

        impl ComponentValue {
            #[must_use]
            pub const fn kind(&self) -> ComponentKind {
                match self {
                    $(Self::$component_variant(_) => ComponentKind::$component_variant,)*
                }
            }
        }
    };
}

macro_rules! count_authoritative_components {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr_2021, $component_variant:ident })*) => {
        <[()]>::len(&[$(count_authoritative_components!(@replace $component_variant)),*])
    };
    (@replace $component_variant:ident) => { () };
}

with_component_schema_entries!(forward_authoritative_components, define_component_kind);
with_component_schema_entries!(forward_authoritative_components, define_component_value);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    LocatedIn,
    InTransit,
    ContainedBy,
    PossessedBy,
    OwnedBy,
    MemberOf,
    LoyalTo,
    SupportDeclaration,
    OfficeHolder,
    ContestsOffice,
    OfficeController,
    HostileTo,
}

impl RelationKind {
    pub const ALL: [Self; 12] = [
        Self::LocatedIn,
        Self::InTransit,
        Self::ContainedBy,
        Self::PossessedBy,
        Self::OwnedBy,
        Self::MemberOf,
        Self::LoyalTo,
        Self::SupportDeclaration,
        Self::OfficeHolder,
        Self::ContestsOffice,
        Self::OfficeController,
        Self::HostileTo,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RelationValue {
    LocatedIn {
        entity: EntityId,
        place: EntityId,
    },
    InTransit {
        entity: EntityId,
    },
    ContainedBy {
        entity: EntityId,
        container: EntityId,
    },
    PossessedBy {
        entity: EntityId,
        holder: EntityId,
    },
    OwnedBy {
        entity: EntityId,
        owner: EntityId,
    },
    MemberOf {
        member: EntityId,
        faction: EntityId,
    },
    LoyalTo {
        subject: EntityId,
        target: EntityId,
        strength: Permille,
    },
    SupportDeclaration {
        supporter: EntityId,
        office: EntityId,
        candidate: EntityId,
    },
    OfficeHolder {
        office: EntityId,
        holder: EntityId,
    },
    ContestsOffice {
        claimant: EntityId,
        office: EntityId,
    },
    OfficeController {
        office: EntityId,
        controller: EntityId,
    },
    HostileTo {
        subject: EntityId,
        target: EntityId,
    },
}

impl RelationValue {
    #[must_use]
    pub const fn kind(&self) -> RelationKind {
        match self {
            Self::LocatedIn { .. } => RelationKind::LocatedIn,
            Self::InTransit { .. } => RelationKind::InTransit,
            Self::ContainedBy { .. } => RelationKind::ContainedBy,
            Self::PossessedBy { .. } => RelationKind::PossessedBy,
            Self::OwnedBy { .. } => RelationKind::OwnedBy,
            Self::MemberOf { .. } => RelationKind::MemberOf,
            Self::LoyalTo { .. } => RelationKind::LoyalTo,
            Self::SupportDeclaration { .. } => RelationKind::SupportDeclaration,
            Self::OfficeHolder { .. } => RelationKind::OfficeHolder,
            Self::ContestsOffice { .. } => RelationKind::ContestsOffice,
            Self::OfficeController { .. } => RelationKind::OfficeController,
            Self::HostileTo { .. } => RelationKind::HostileTo,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EntityDelta {
    Created { entity: EntityId, kind: EntityKind },
    Archived { entity: EntityId, kind: EntityKind },
}

/// Compact structural diff for a specific component type.
///
/// Each variant wraps the diff type for one component kind. New component
/// diff types are added as new variants here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentDiff {
    BeliefStore(BeliefStoreDiff),
}

impl ComponentDiff {
    /// Apply this diff to a `ComponentValue`, producing the updated value.
    ///
    /// Panics if the diff variant does not match the component value variant.
    #[must_use]
    pub fn apply_to_component_value(&self, base: &ComponentValue) -> ComponentValue {
        match self {
            ComponentDiff::BeliefStore(diff) => {
                let ComponentValue::AgentBeliefStore(store) = base else {
                    panic!("BeliefStore diff applied to non-AgentBeliefStore ComponentValue");
                };
                ComponentValue::AgentBeliefStore(diff.clone().apply(store))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ComponentDelta {
    Set {
        entity: EntityId,
        component_kind: ComponentKind,
        before: Option<ComponentValue>,
        after: ComponentValue,
    },
    CompactSet {
        entity: EntityId,
        component_kind: ComponentKind,
        diff: ComponentDiff,
    },
    Removed {
        entity: EntityId,
        component_kind: ComponentKind,
        before: ComponentValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RelationDelta {
    Added {
        relation_kind: RelationKind,
        relation: RelationValue,
    },
    Removed {
        relation_kind: RelationKind,
        relation: RelationValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantityDelta {
    Changed {
        entity: EntityId,
        commodity: CommodityKind,
        before: Quantity,
        after: Quantity,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReservationDelta {
    Created { reservation: ReservationRecord },
    Released { reservation: ReservationRecord },
}

// Component deltas are intentionally stored inline so event-log deltas remain
// value-semantic and allocation-free on the hot commit path.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StateDelta {
    Entity(EntityDelta),
    Component(ComponentDelta),
    Relation(RelationDelta),
    Quantity(QuantityDelta),
    Reservation(ReservationDelta),
}

#[cfg(test)]
mod tests {
    use super::{
        ComponentDelta, ComponentDiff, ComponentKind, ComponentValue, EntityDelta, QuantityDelta,
        RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta,
    };
    use crate::{
        AcquisitionExhaustionTracker, ActionDefId, AgendaProfile, AgentBeliefStore, AgentData,
        AgentSchemaContextProfile, ArtifactHeader, ArtifactKind, ArtifactPostingProfile,
        BanditCamp, BanditFactionPolicy, BeliefConfidencePolicy, BelievedEntityState, BodyPart,
        BountyTarget, BountyTerms, CarryCapacity, CognitiveArchetype, CognitiveArchetypeComponent,
        CognitiveProfile, CombatProfile, CombatStance, CommodityKind, CommunicationProfile,
        Container, ContentionIntents, ContentionPolicy, ContentionQueue, ControlSource, DeadAt,
        DeprivationExposure, DeprivationKind, DisposalProfile, DiversificationProfile,
        DriveEscalationProfile, DriveThresholds, EntityId, EntityKind, EpistemicDispositionProfile,
        EventId, ExecutionBudget, ExpectationStore, ExplorationProfile, FactionData, FrameState,
        GoalKey, GoalKind, GroundComfortTag, GroundSince, HomeostaticNeeds, HypothesisKind,
        InTransitOnEdge, InstitutionalClaim, InstitutionalRecordEntry, IntentionDispositionProfile,
        IntentionDomain, IntentionDomainTag, IntentionFrame, ItemLot, JusticeDispositionProfile,
        KnownRecipes, LastHarvestTrace, LastProactiveExplorationTick, LastSeenMemory,
        LatrineFullness, LawAbidingProfile, LoadUnits, LotOperation, MetabolismProfile, Name,
        NoticeContent, NoticeTopic, ObligationExecutionTracker, ObligationSatiationProfile,
        OfficeData, OfficeForceProfile, OfficeForceState, OfficePatrolDuty, PatrolProfile,
        PatrolRoute, PerceptionProfile, PerceptionSource, Permille, PlaceDirtiness,
        PlaceVisibilityProfile, PlaceVisitRecord, PortfolioWeightsProfile, ProductionJob,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, ProofRequirement, ProvenanceEntry,
        PursuitProfile, Quantity, QueuedContentionIntent, RecordData, RecordEntryId, RecordKind,
        ReservationId, ReservationRecord, ResourceExtractionQueues, ResourceSource,
        RewardEncumbrance, RewardSource, RiskWeightProfile, RoutePreferenceProfile, SaleListing,
        SceneEvidence, ShelterTag, SleepEpisode, SleepQualityProfile, SleepRecoveryModifier,
        StockAssignment, StockAssignmentKind, StockStoragePolicy, SurveyMemory, SurveyRecord,
        TellProfile, TestimonyTrustProfile, TheftDispositionProfile, Tick, TickRange, TravelEdgeId,
        UniqueItem, UniqueItemKind, ViolationDispositionProfile, ViolationMemory, WakeCondition,
        WashBasinState, WorkstationMarker, WorkstationTag, Wound, WoundCause, WoundList,
        test_utils::{
            sample_blocker_memory, sample_commodity_valuation_profile,
            sample_contention_disposition_profile, sample_demand_memory, sample_discrepancy_memory,
            sample_learned_opportunity_memory, sample_memory_capacity_profile,
            sample_merchandise_profile, sample_preference_profile, sample_repair_memory,
            sample_route_experience, sample_source_reliability, sample_substitute_preferences,
            sample_trade_disposition_profile, sample_utility_profile,
        },
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn reservation_record() -> ReservationRecord {
        ReservationRecord {
            id: ReservationId(7),
            entity: entity(4),
            reserver: entity(5),
            range: TickRange::new(Tick(8), Tick(12)).unwrap(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn component_samples() -> Vec<ComponentValue> {
        vec![
            ComponentValue::Name(Name("Aster".to_string())),
            ComponentValue::ArtifactPostingProfile(ArtifactPostingProfile::default()),
            ComponentValue::AgentData(AgentData {
                control_source: ControlSource::Ai,
            }),
            ComponentValue::WoundList(WoundList {
                wounds: vec![Wound {
                    id: crate::WoundId(1),
                    body_part: BodyPart::Head,
                    cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                    severity: Permille::new(900).unwrap(),
                    inflicted_at: Tick(5),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            }),
            ComponentValue::SurveyMemory(SurveyMemory {
                entries: vec![SurveyRecord {
                    place: entity(7),
                    hypothesis: HypothesisKind::MayContainCommodity {
                        commodity: CommodityKind::Apple,
                    },
                    found: false,
                    confidence: Permille::new(850).unwrap(),
                    recorded_tick: Tick(6),
                }],
            }),
            ComponentValue::CombatProfile(CombatProfile::new(
                Permille::new(1000).unwrap(),
                Permille::new(700).unwrap(),
                Permille::new(640).unwrap(),
                Permille::new(590).unwrap(),
                Permille::new(75).unwrap(),
                Permille::new(22).unwrap(),
                Permille::new(17).unwrap(),
                Permille::new(130).unwrap(),
                Permille::new(28).unwrap(),
                std::num::NonZeroU32::new(6).unwrap(),
                std::num::NonZeroU32::new(10).unwrap(),
            )),
            ComponentValue::DeadAt(DeadAt {
                tick: Tick(18),
                cause: crate::DeathCause::CombatWounds,
            }),
            ComponentValue::CombatStance(CombatStance::Defending),
            ComponentValue::ContentionDispositionProfile(sample_contention_disposition_profile()),
            ComponentValue::ContentionPolicy(ContentionPolicy {
                grant_hold_ticks: std::num::NonZeroU32::new(4).unwrap(),
                auto_promote: true,
                max_waiters: Some(2),
            }),
            ComponentValue::ContentionQueue(ContentionQueue::default()),
            ComponentValue::TheftDispositionProfile(TheftDispositionProfile {
                steal_duration_ticks: std::num::NonZeroU32::new(5).unwrap(),
                theft_motive_weight: Permille::new(620).unwrap(),
                witness_risk_penalty: Permille::new(180).unwrap(),
            }),
            ComponentValue::JusticeDispositionProfile(JusticeDispositionProfile {
                accusation_motive_weight: Permille::new(700).unwrap(),
                fine_severity: Permille::new(450).unwrap(),
            }),
            ComponentValue::UtilityProfile(sample_utility_profile()),
            ComponentValue::CommodityValuationProfile(sample_commodity_valuation_profile()),
            ComponentValue::RouteExperience(sample_route_experience()),
            ComponentValue::SourceReliability(sample_source_reliability()),
            ComponentValue::PreferenceProfile(sample_preference_profile()),
            ComponentValue::TestimonyTrustProfile(TestimonyTrustProfile::default()),
            ComponentValue::RoutePreferenceProfile(RoutePreferenceProfile::default()),
            ComponentValue::RiskWeightProfile(RiskWeightProfile::default()),
            ComponentValue::LawAbidingProfile(LawAbidingProfile::default()),
            ComponentValue::DriveEscalationProfile(DriveEscalationProfile::default()),
            ComponentValue::PatrolRoute(PatrolRoute {
                assigned_places: vec![entity(26), entity(27), entity(28)],
                current_index: 1,
            }),
            ComponentValue::PatrolProfile(PatrolProfile {
                base_dwell_ticks: 8,
                dwell_vigilance_scale_ticks: 8,
                vigilance: Permille::new(650).unwrap(),
                route_adaptation_sensitivity: Permille::new(375).unwrap(),
                patrol_motive_weight: Permille::new(550).unwrap(),
            }),
            ComponentValue::OfficePatrolDuty(OfficePatrolDuty {
                issuing_office: entity(32),
                delegate: None,
                assignee: entity(4),
                assigned_places: vec![entity(26), entity(27)],
                created_tick: Tick(4),
                renewal_due_tick: Tick(9),
                grace_ticks: 3,
                lifecycle: crate::OfficePatrolDutyLifecycle::Active,
                provenance: crate::OfficePatrolDutyProvenance::IssuedByOffice { tick: Tick(4) },
            }),
            ComponentValue::OfficeData(OfficeData {
                title: "Granary Chair".to_string(),
                seat: entity(32),
                jurisdiction: BTreeSet::from([entity(32)]),
                succession_law: crate::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 12,
                vacancy_since: Some(Tick(6)),
            }),
            ComponentValue::OfficeForceProfile(OfficeForceProfile {
                uncontested_hold_ticks: std::num::NonZeroU32::new(9).unwrap(),
                vacancy_claim_grace_ticks: std::num::NonZeroU32::new(4).unwrap(),
                challenger_presence_grace_ticks: std::num::NonZeroU32::new(2).unwrap(),
            }),
            ComponentValue::OfficeForceState(OfficeForceState {
                control_since: Some(Tick(7)),
                challenged_since: Some(Tick(7)),
                contested_since: Some(Tick(8)),
                last_uncontested_tick: Some(Tick(10)),
            }),
            ComponentValue::FactionData(FactionData {
                name: "River Pact".to_string(),
                purpose: crate::FactionPurpose::Political,
            }),
            ComponentValue::RecordData(RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: entity(33),
                issuer: entity(34),
                consultation_ticks: 5,
                max_entries_per_consult: 7,
                entries: vec![InstitutionalRecordEntry {
                    entry_id: RecordEntryId(0),
                    claim: InstitutionalClaim::OfficeHolder {
                        office: entity(35),
                        holder: Some(entity(36)),
                        effective_tick: Tick(8),
                    },
                    recorded_tick: Tick(9),
                    supersedes: None,
                }],
                next_entry_id: 1,
            }),
            ComponentValue::ArtifactHeader(ArtifactHeader::posted_active(
                ArtifactKind::Bounty,
                entity(37),
                Some(entity(38)),
                Tick(10),
                Some(Tick(25)),
                Some(entity(39)),
                entity(42),
            )),
            ComponentValue::BountyTerms(BountyTerms {
                target: BountyTarget::EliminateEntity { target: entity(40) },
                proof_requirement: ProofRequirement::PhysicalEvidence,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(11),
                reward_source: RewardSource::ReservedLot { lot: entity(41) },
                claim_place: entity(42),
            }),
            ComponentValue::NoticeContent(NoticeContent {
                topic: NoticeTopic::ThreatWarning { place: entity(43) },
            }),
            ComponentValue::BlockerMemory(sample_blocker_memory()),
            ComponentValue::DiscrepancyMemory(sample_discrepancy_memory()),
            ComponentValue::RepairMemory(sample_repair_memory()),
            ComponentValue::LearnedOpportunityMemory(sample_learned_opportunity_memory()),
            ComponentValue::MemoryCapacityProfile(sample_memory_capacity_profile()),
            ComponentValue::AgentBeliefStore(AgentBeliefStore {
                entity_claims: BTreeMap::new(),
                next_claim_id: crate::ClaimId(0),
                known_entities: BTreeMap::from([(
                    entity(18),
                    BelievedEntityState {
                        believed_kind: None,
                        last_known_place: Some(entity(19)),
                        last_known_inventory: BTreeMap::from([
                            (CommodityKind::Apple, Quantity(2)),
                            (CommodityKind::Water, Quantity(1)),
                        ]),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        believed_activity: None,
                        believed_artifact: None,
                        believed_contention: None,
                        believed_evidence: None,
                        ..BelievedEntityState::single_observation_defaults(
                            Tick(14),
                            PerceptionSource::DirectObservation,
                        )
                    },
                )]),
                social_observations: Vec::new(),
                observation_omission_log: crate::ObservationOmissionLog::default(),
                told_beliefs: BTreeMap::new(),
                heard_beliefs: BTreeMap::new(),
                asked_witnesses: BTreeMap::new(),
                place_visits: BTreeMap::from([(
                    entity(24),
                    PlaceVisitRecord {
                        ticks_present: 9,
                        last_arrival_tick: Tick(13),
                        visit_count: 2,
                    },
                )]),
                institutional_beliefs: BTreeMap::from([(
                    crate::InstitutionalBeliefKey::OfficeHolderOf { office: entity(20) },
                    vec![crate::BelievedInstitutionalClaim {
                        claim: crate::InstitutionalClaim::OfficeHolder {
                            office: entity(20),
                            holder: Some(entity(21)),
                            effective_tick: Tick(15),
                        },
                        source: crate::InstitutionalKnowledgeSource::RecordConsultation {
                            record: entity(22),
                            entry_id: crate::RecordEntryId(4),
                        },
                        learned_tick: Tick(16),
                        learned_at: Some(entity(23)),
                    }],
                )]),
            }),
            ComponentValue::ExpectationStore(ExpectationStore {
                records: BTreeMap::from([(
                    crate::ExpectationId(3),
                    crate::ExpectationRecord {
                        id: crate::ExpectationId(3),
                        owner: entity(44),
                        subject: entity(45),
                        expected_place: entity(46),
                        deadline_tick: Tick(17),
                        grace_ticks: 6,
                        basis: crate::ExpectationBasis::RoutineReturn,
                        state: crate::ExpectationState::Overdue,
                        created_tick: Tick(11),
                    },
                )]),
                next_expectation_id: crate::ExpectationId(4),
            }),
            ComponentValue::LastSeenMemory(LastSeenMemory {
                records: BTreeMap::from([(
                    entity(47),
                    crate::LastSeenRecord {
                        subject: entity(47),
                        place: entity(48),
                        observed_tick: Tick(18),
                        source: entity(49),
                        provenance: crate::LastSeenProvenance::Hearsay {
                            original_observer: entity(50),
                            chain_depth: 1,
                        },
                    },
                )]),
                capacity: 9,
            }),
            ComponentValue::PerceptionProfile(PerceptionProfile {
                observation_fidelity: Permille::new(920).unwrap(),
                confidence_policy: BeliefConfidencePolicy::default(),
                institutional_memory_capacity: 24,
                consultation_speed_factor: Permille::new(600).unwrap(),
                contradiction_tolerance: Permille::new(350).unwrap(),
                entity_activation_threshold: Permille::new(100).unwrap(),
                claim_confidence_threshold: Permille::new(50).unwrap(),
                observation_buffer_capacity: 5,
                observation_budget: 24,
                salience_policy: crate::SaliencePolicy::default(),
                omission_log_capacity: crate::default_omission_log_capacity(),
                opportunity_floor_permille: Permille::new(100).unwrap(),
                need_salience_boost: Permille::new(500).unwrap(),
                need_salience_urgency_threshold: Permille::new(500).unwrap(),
            }),
            ComponentValue::TellProfile(TellProfile {
                max_tell_candidates: 4,
                max_relay_chain_len: 2,
                conversation_memory_capacity: 9,
                conversation_memory_retention_ticks: 28,
            }),
            ComponentValue::CommunicationProfile(CommunicationProfile {
                alarm_acceptance: Permille::new(980).unwrap(),
                testimony_acceptance: Permille::new(830).unwrap(),
                gossip_acceptance: Permille::new(540).unwrap(),
            }),
            ComponentValue::CognitiveProfile(CognitiveProfile {
                max_candidates_per_expansion: 180,
                max_plan_depth: 9,
                max_travel_candidates_per_expansion: None,
                snapshot_travel_horizon: 5,
                max_node_expansions: 320,
                switch_margin: Permille::new(175).unwrap(),
                planning_switch_margin: Permille::new(225).unwrap(),
                transient_block_ticks: 12,
                structural_block_ticks: 250,
                stale_belief_backoff_ticks: 31,
                contradicted_belief_backoff_ticks: 61,
                improper_state_backoff_ticks: 3,
                missing_observation_backoff_ticks: 21,
                no_legal_binding_backoff_ticks: 121,
                counterparty_refusal_backoff_ticks: 41,
                route_unknown_backoff_ticks: 201,
                route_segment_blocker_ticks: 241,
                counterparty_blocker_ticks: 361,
                search_exhaustion_backoff_ticks: 101,
                partial_drift_backoff_ticks: 5,
                expectation_tolerance_ticks: 7,
                guard_min_confidence_ceiling: Permille::new(875).unwrap(),
                repair_memory_ticks: 144,
                learned_opportunity_memory_ticks: 72,
                survey_memory_capacity: 18,
                survey_memory_retention_ticks: 360,
                initial_cooldown_ticks: 7,
                max_cooldown_ticks: 90,
                landmark_extraction_depth: 3,
                use_ff_heuristic: true,
                decision_history_alternatives: 5,
                detour_budget_permille: Permille::new(150).unwrap(),
                compile_opportunity_cap: 16,
                repair_budget_fraction: Permille::new(250).unwrap(),
                causal_links_per_step_cap: 8,
            }),
            ComponentValue::CognitiveArchetypeComponent(CognitiveArchetypeComponent {
                archetype: CognitiveArchetype::Greedy,
            }),
            ComponentValue::PortfolioWeightsProfile(PortfolioWeightsProfile {
                need_survival: Permille::new(990).unwrap(),
                pain_care: Permille::new(880).unwrap(),
                obligation_duty: Permille::new(770).unwrap(),
                economic_opportunity: Permille::new(660).unwrap(),
                social_motive: Permille::new(550).unwrap(),
                max_plans_normal: 6,
                max_plans_emergency: 4,
                max_plans_idle: 7,
            }),
            ComponentValue::AgentSchemaContextProfile(AgentSchemaContextProfile::default()),
            ComponentValue::AgendaProfile(AgendaProfile {
                pending_capacity: 16,
                suspended_capacity: 8,
                revive_cooldown_ticks: 4,
            }),
            ComponentValue::ExplorationProfile(ExplorationProfile {
                curiosity_weight: Permille::new(650).unwrap(),
                need_activation_threshold: Permille::new(450).unwrap(),
                frontier_depth: 3,
                acquisition_failure_threshold: 4,
                exploration_arrival_boost: Permille::new(550).unwrap(),
                max_consecutive_explorations: 4,
                visit_lookback_ticks: 240,
                negative_survey_damping_window: 160,
                negative_survey_damping_strength: Permille::new(700).unwrap(),
                consecutive_exploration_count: 1,
            }),
            ComponentValue::DiversificationProfile(DiversificationProfile {
                base_curiosity: Permille::new(620).unwrap(),
                comfort_threshold: Permille::new(425).unwrap(),
                curiosity_buildup_rate: Permille::new(7).unwrap(),
                exploration_cooldown_ticks: 90,
                familiarity_per_visit: Permille::new(140).unwrap(),
                familiarity_recovery_per_tick: Permille::new(3).unwrap(),
                familiarity_floor: Permille::new(60).unwrap(),
                max_exploration_hops: 4,
            }),
            ComponentValue::LastProactiveExplorationTick(LastProactiveExplorationTick(Some(Tick(
                19,
            )))),
            ComponentValue::ObligationSatiationProfile(ObligationSatiationProfile {
                satiation_threshold: 3,
                window_ticks: 72,
                decay_per_execution: Permille::new(180).unwrap(),
                satiation_floor: Permille::new(75).unwrap(),
            }),
            ComponentValue::ObligationExecutionTracker(ObligationExecutionTracker {
                completion_ticks: vec![Tick(6), Tick(11)],
            }),
            ComponentValue::DisposalProfile(DisposalProfile {
                capacity_strain_threshold: Permille::new(850).unwrap(),
            }),
            ComponentValue::ExecutionBudget(ExecutionBudget::new(11, 4, 3)),
            ComponentValue::AcquisitionExhaustionTracker(AcquisitionExhaustionTracker::default()),
            ComponentValue::DriveThresholds(DriveThresholds::default()),
            ComponentValue::HomeostaticNeeds(HomeostaticNeeds::new(
                Permille::new(100).unwrap(),
                Permille::new(200).unwrap(),
                Permille::new(300).unwrap(),
                Permille::new(400).unwrap(),
                Permille::new(500).unwrap(),
            )),
            ComponentValue::DeprivationExposure(DeprivationExposure {
                hunger_critical_ticks: 1,
                thirst_critical_ticks: 2,
                fatigue_critical_ticks: 3,
                bladder_critical_ticks: 4,
                dirtiness_critical_ticks: 5,
            }),
            ComponentValue::MetabolismProfile(MetabolismProfile::default()),
            ComponentValue::CarryCapacity(CarryCapacity(LoadUnits(14))),
            ComponentValue::KnownRecipes(KnownRecipes::with([
                crate::RecipeId(2),
                crate::RecipeId(7),
            ])),
            ComponentValue::DemandMemory(sample_demand_memory()),
            ComponentValue::TradeDispositionProfile(sample_trade_disposition_profile()),
            ComponentValue::MerchandiseProfile(sample_merchandise_profile()),
            ComponentValue::SubstitutePreferences(sample_substitute_preferences()),
            ComponentValue::WorkstationMarker(WorkstationMarker(WorkstationTag::Forge)),
            ComponentValue::ResourceSource(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(6),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: Some(std::num::NonZeroU32::new(4).unwrap()),
                last_regeneration_tick: Some(Tick(12)),
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            }),
            ComponentValue::LastHarvestTrace(LastHarvestTrace {
                entries: vec![crate::HarvestTraceEntry {
                    harvester: entity(50),
                    tick: Tick(20),
                    quantity: 2,
                    partial: false,
                }],
            }),
            ComponentValue::ResourceExtractionQueues(ResourceExtractionQueues {
                queues: vec![ContentionQueue::default(), ContentionQueue::default()],
            }),
            ComponentValue::ProductionOutputOwnershipPolicy(ProductionOutputOwnershipPolicy {
                output_owner: ProductionOutputOwner::ProducerOwner,
            }),
            ComponentValue::BanditCamp(BanditCamp {
                faction: entity(40),
                supplies: entity(41),
                empty_since_tick: Some(Tick(9)),
            }),
            ComponentValue::SceneEvidence(SceneEvidence {
                evidence: vec![crate::EvidenceEntry {
                    id: crate::EvidenceEntryId(1),
                    kind: crate::EvidenceKind::DisturbanceMarker {
                        place: entity(42),
                        kind: crate::DisturbanceKind::ForcedEntry,
                        created_at: Tick(16),
                    },
                    created_at: Tick(16),
                    decay_ticks: 50,
                }],
                next_entry_id: 2,
            }),
            ComponentValue::PlaceVisibilityProfile(PlaceVisibilityProfile {
                base_concealment: Permille::new(375).unwrap(),
            }),
            ComponentValue::SleepQualityProfile(SleepQualityProfile {
                shelter: ShelterTag::Shelter,
                ground_comfort: GroundComfortTag::Soft,
                recovery_modifier: SleepRecoveryModifier::new(850),
            }),
            ComponentValue::PlaceDirtiness(PlaceDirtiness {
                value: Permille::new(500).unwrap(),
                decay_per_tick: Permille::new(2).unwrap(),
                dirtiness_per_use: Permille::new(80).unwrap(),
            }),
            ComponentValue::LatrineFullness(LatrineFullness {
                fill: Permille::new(650).unwrap(),
                fill_per_use: Permille::new(80).unwrap(),
                critical_threshold: Permille::new(800).unwrap(),
            }),
            ComponentValue::WashBasinState(WashBasinState {
                clean_water_units: 4,
                max_clean_water: 12,
                refill_per_tick: 2,
                units_per_full_wash: 3,
                dirtiness_level: Permille::new(250).unwrap(),
                dirtiness_per_use: Permille::new(50).unwrap(),
            }),
            ComponentValue::BanditFactionPolicy(BanditFactionPolicy {
                min_regroup_count: 3,
                establishment_duration_ticks: std::num::NonZeroU32::new(14).unwrap(),
                abandonment_grace_ticks: std::num::NonZeroU32::new(5).unwrap(),
                flee_wound_threshold: Permille::new(675).unwrap(),
                rally_place: Some(entity(42)),
            }),
            ComponentValue::ProductionJob(ProductionJob {
                recipe_id: crate::RecipeId(3),
                worker: entity(24),
                staged_inputs_container: entity(25),
                progress_ticks: 9,
            }),
            ComponentValue::InTransitOnEdge(InTransitOnEdge {
                edge_id: TravelEdgeId(4),
                origin: entity(30),
                destination: entity(31),
                departure_tick: Tick(13),
                arrival_tick: Tick(21),
            }),
            ComponentValue::ContentionIntents(ContentionIntents {
                intents: BTreeMap::from([(
                    entity(40),
                    QueuedContentionIntent {
                        goal_key: GoalKey::from(GoalKind::Sleep),
                        intended_action: ActionDefId(9),
                    },
                )]),
            }),
            ComponentValue::IntentionFrame(IntentionFrame {
                goal: GoalKey::from(GoalKind::Sleep),
                domain: IntentionDomain::Generic,
                assumptions: vec![],
                state: FrameState::Active,
                established_at: Tick(5),
                last_progress_tick: None,
                stalled_ticks: 0,
                patience_limit: 30,
                motive_refs: Vec::new(),
                resume_conditions: Vec::new(),
                abandon_conditions: Vec::new(),
                explicit_claims: Vec::new(),
                causal_links: Vec::new(),
            }),
            ComponentValue::SleepEpisode(SleepEpisode {
                place: entity(31),
                start_tick: Tick(5),
                intended_min_ticks: std::num::NonZeroU32::new(3).unwrap(),
                intended_max_ticks: std::num::NonZeroU32::new(30).unwrap(),
                target_recovery: Permille::new(650).unwrap(),
                accumulated_recovery: Permille::new(125).unwrap(),
                recovery_modifier: SleepRecoveryModifier::new(850),
                wake_conditions: vec![WakeCondition::TargetRecoveryReached],
            }),
            ComponentValue::IntentionDispositionProfile(IntentionDispositionProfile {
                domain_patience: BTreeMap::from([(
                    IntentionDomainTag::Travel,
                    std::num::NonZeroU32::new(50).unwrap(),
                )]),
                default_patience_ticks: std::num::NonZeroU32::new(30).unwrap(),
                commitment_switch_margin: Permille::new(200).unwrap(),
            }),
            ComponentValue::ViolationMemory(ViolationMemory::default()),
            ComponentValue::ViolationDispositionProfile(ViolationDispositionProfile {
                investigation_duration_ticks: std::num::NonZeroU32::new(3).unwrap(),
                violation_memory_retention_ticks: 50,
                investigation_motive_weight: Permille::new(500).unwrap(),
                ownership_motive_bonus: Permille::new(200).unwrap(),
            }),
            ComponentValue::EpistemicDispositionProfile(EpistemicDispositionProfile {
                stale_evidence_barrier_threshold: Permille::new(400).unwrap(),
                witness_query_duration_ticks: std::num::NonZeroU32::new(2).unwrap(),
                ask_memory_retention_ticks: 12,
                witness_recency_preference: Permille::new(500).unwrap(),
            }),
            ComponentValue::PursuitProfile(PursuitProfile {
                min_location_confidence: Permille::new(600).unwrap(),
                max_pursuit_travel_ticks: std::num::NonZeroU32::new(10).unwrap(),
            }),
            ComponentValue::ItemLot(ItemLot {
                commodity: CommodityKind::Grain,
                quantity: Quantity(11),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(3),
                    event_id: Some(EventId(2)),
                    operation: LotOperation::Produced,
                    related_lot: Some(entity(9)),
                    amount: Quantity(4),
                }],
            }),
            ComponentValue::UniqueItem(UniqueItem {
                kind: UniqueItemKind::Artifact,
                name: Some("Seal".to_string()),
                metadata: BTreeMap::from([("origin".to_string(), "court".to_string())]),
            }),
            ComponentValue::GroundSince(GroundSince(Tick(22))),
            ComponentValue::Container(Container {
                capacity: LoadUnits(25),
                allowed_commodities: Some(BTreeSet::from([
                    CommodityKind::Apple,
                    CommodityKind::Water,
                ])),
                allows_unique_items: true,
                allows_nested_containers: false,
            }),
            ComponentValue::SaleListing(SaleListing {
                listed_at: Tick(10),
            }),
            ComponentValue::RewardEncumbrance(RewardEncumbrance {
                reservations: vec![crate::RewardReservation {
                    bounty_artifact: entity(103),
                    commodity: CommodityKind::Coin,
                    quantity: Quantity(19),
                }],
            }),
            ComponentValue::StockStoragePolicy(StockStoragePolicy {
                stock_container: crate::test_utils::entity_id(100, 1),
                display_container: Some(crate::test_utils::entity_id(101, 1)),
            }),
            ComponentValue::StockAssignment(StockAssignment {
                facility: crate::test_utils::entity_id(102, 1),
                kind: StockAssignmentKind::Stored,
            }),
        ]
    }

    fn relation_samples() -> Vec<RelationValue> {
        vec![
            RelationValue::LocatedIn {
                entity: entity(1),
                place: entity(2),
            },
            RelationValue::InTransit { entity: entity(3) },
            RelationValue::ContainedBy {
                entity: entity(4),
                container: entity(5),
            },
            RelationValue::PossessedBy {
                entity: entity(6),
                holder: entity(7),
            },
            RelationValue::OwnedBy {
                entity: entity(8),
                owner: entity(9),
            },
            RelationValue::MemberOf {
                member: entity(10),
                faction: entity(11),
            },
            RelationValue::LoyalTo {
                subject: entity(12),
                target: entity(13),
                strength: Permille::new(650).unwrap(),
            },
            RelationValue::SupportDeclaration {
                supporter: entity(14),
                office: entity(15),
                candidate: entity(16),
            },
            RelationValue::OfficeHolder {
                office: entity(17),
                holder: entity(18),
            },
            RelationValue::ContestsOffice {
                claimant: entity(19),
                office: entity(20),
            },
            RelationValue::OfficeController {
                office: entity(21),
                controller: entity(22),
            },
            RelationValue::HostileTo {
                subject: entity(23),
                target: entity(24),
            },
        ]
    }

    fn assert_traits<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}
    fn assert_kind_traits<
        T: Copy + Clone + Debug + Eq + Ord + std::hash::Hash + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn delta_types_satisfy_required_traits() {
        assert_kind_traits::<ComponentKind>();
        assert_kind_traits::<RelationKind>();
        assert_traits::<ComponentValue>();
        assert_traits::<RelationValue>();
        assert_traits::<EntityDelta>();
        assert_traits::<ComponentDelta>();
        assert_traits::<RelationDelta>();
        assert_traits::<QuantityDelta>();
        assert_traits::<ReservationDelta>();
        assert_traits::<StateDelta>();
    }

    #[test]
    fn component_kind_variants_match_authoritative_components() {
        assert_eq!(
            ComponentKind::ALL,
            [
                ComponentKind::Name,
                ComponentKind::ArtifactPostingProfile,
                ComponentKind::AgentData,
                ComponentKind::WoundList,
                ComponentKind::SurveyMemory,
                ComponentKind::CombatProfile,
                ComponentKind::DeadAt,
                ComponentKind::CombatStance,
                ComponentKind::ContentionDispositionProfile,
                ComponentKind::TheftDispositionProfile,
                ComponentKind::JusticeDispositionProfile,
                ComponentKind::UtilityProfile,
                ComponentKind::CommodityValuationProfile,
                ComponentKind::RouteExperience,
                ComponentKind::SourceReliability,
                ComponentKind::PreferenceProfile,
                ComponentKind::TestimonyTrustProfile,
                ComponentKind::RoutePreferenceProfile,
                ComponentKind::RiskWeightProfile,
                ComponentKind::LawAbidingProfile,
                ComponentKind::PatrolRoute,
                ComponentKind::PatrolProfile,
                ComponentKind::OfficePatrolDuty,
                ComponentKind::OfficeData,
                ComponentKind::OfficeForceProfile,
                ComponentKind::OfficeForceState,
                ComponentKind::RewardEncumbrance,
                ComponentKind::FactionData,
                ComponentKind::RecordData,
                ComponentKind::ArtifactHeader,
                ComponentKind::BountyTerms,
                ComponentKind::NoticeContent,
                ComponentKind::BlockerMemory,
                ComponentKind::DiscrepancyMemory,
                ComponentKind::RepairMemory,
                ComponentKind::LearnedOpportunityMemory,
                ComponentKind::MemoryCapacityProfile,
                ComponentKind::AgentBeliefStore,
                ComponentKind::ExpectationStore,
                ComponentKind::LastSeenMemory,
                ComponentKind::PerceptionProfile,
                ComponentKind::TellProfile,
                ComponentKind::CommunicationProfile,
                ComponentKind::CognitiveProfile,
                ComponentKind::CognitiveArchetypeComponent,
                ComponentKind::PortfolioWeightsProfile,
                ComponentKind::AgentSchemaContextProfile,
                ComponentKind::AgendaProfile,
                ComponentKind::AcquisitionExhaustionTracker,
                ComponentKind::ExplorationProfile,
                ComponentKind::DiversificationProfile,
                ComponentKind::LastProactiveExplorationTick,
                ComponentKind::ObligationSatiationProfile,
                ComponentKind::ObligationExecutionTracker,
                ComponentKind::DisposalProfile,
                ComponentKind::ExecutionBudget,
                ComponentKind::DriveEscalationProfile,
                ComponentKind::DriveThresholds,
                ComponentKind::HomeostaticNeeds,
                ComponentKind::DeprivationExposure,
                ComponentKind::MetabolismProfile,
                ComponentKind::CarryCapacity,
                ComponentKind::KnownRecipes,
                ComponentKind::DemandMemory,
                ComponentKind::TradeDispositionProfile,
                ComponentKind::MerchandiseProfile,
                ComponentKind::SubstitutePreferences,
                ComponentKind::ContentionPolicy,
                ComponentKind::ContentionQueue,
                ComponentKind::WorkstationMarker,
                ComponentKind::ResourceSource,
                ComponentKind::LastHarvestTrace,
                ComponentKind::ResourceExtractionQueues,
                ComponentKind::ProductionOutputOwnershipPolicy,
                ComponentKind::BanditCamp,
                ComponentKind::SceneEvidence,
                ComponentKind::PlaceVisibilityProfile,
                ComponentKind::SleepQualityProfile,
                ComponentKind::PlaceDirtiness,
                ComponentKind::LatrineFullness,
                ComponentKind::WashBasinState,
                ComponentKind::BanditFactionPolicy,
                ComponentKind::ProductionJob,
                ComponentKind::InTransitOnEdge,
                ComponentKind::ContentionIntents,
                ComponentKind::IntentionFrame,
                ComponentKind::SleepEpisode,
                ComponentKind::IntentionDispositionProfile,
                ComponentKind::ViolationMemory,
                ComponentKind::ViolationDispositionProfile,
                ComponentKind::EpistemicDispositionProfile,
                ComponentKind::PursuitProfile,
                ComponentKind::ItemLot,
                ComponentKind::UniqueItem,
                ComponentKind::GroundSince,
                ComponentKind::Container,
                ComponentKind::SaleListing,
                ComponentKind::StockStoragePolicy,
                ComponentKind::StockAssignment,
            ]
        );
    }

    #[test]
    fn component_value_reports_matching_component_kind() {
        let samples = component_samples();

        assert_eq!(samples.len(), ComponentKind::ALL.len());
        for sample in samples {
            assert!(ComponentKind::ALL.contains(&sample.kind()));
        }
    }

    #[test]
    fn relation_kind_variants_match_semantic_relation_families() {
        assert_eq!(
            RelationKind::ALL,
            [
                RelationKind::LocatedIn,
                RelationKind::InTransit,
                RelationKind::ContainedBy,
                RelationKind::PossessedBy,
                RelationKind::OwnedBy,
                RelationKind::MemberOf,
                RelationKind::LoyalTo,
                RelationKind::SupportDeclaration,
                RelationKind::OfficeHolder,
                RelationKind::ContestsOffice,
                RelationKind::OfficeController,
                RelationKind::HostileTo,
            ]
        );
    }

    #[test]
    fn relation_value_reports_matching_relation_kind() {
        let samples = relation_samples();

        assert_eq!(samples.len(), RelationKind::ALL.len());
        for sample in samples {
            assert!(RelationKind::ALL.contains(&sample.kind()));
        }
    }

    #[test]
    fn entity_delta_stores_entity_id_and_kind() {
        let created = EntityDelta::Created {
            entity: entity(1),
            kind: EntityKind::Agent,
        };
        let archived = EntityDelta::Archived {
            entity: entity(2),
            kind: EntityKind::Office,
        };

        assert!(matches!(
            created,
            EntityDelta::Created {
                entity: created_entity,
                kind: EntityKind::Agent
            } if created_entity == entity(1)
        ));
        assert!(matches!(
            archived,
            EntityDelta::Archived {
                entity: archived_entity,
                kind: EntityKind::Office
            } if archived_entity == entity(2)
        ));
    }

    #[test]
    fn component_delta_stores_typed_before_after_snapshots() {
        let before = ComponentValue::Name(Name("Old".to_string()));
        let after = ComponentValue::Name(Name("New".to_string()));
        let set = ComponentDelta::Set {
            entity: entity(3),
            component_kind: ComponentKind::Name,
            before: Some(before.clone()),
            after: after.clone(),
        };
        let removed = ComponentDelta::Removed {
            entity: entity(4),
            component_kind: ComponentKind::RewardEncumbrance,
            before: ComponentValue::RewardEncumbrance(RewardEncumbrance {
                reservations: vec![crate::RewardReservation {
                    bounty_artifact: entity(5),
                    commodity: CommodityKind::Coin,
                    quantity: Quantity(7),
                }],
            }),
        };

        assert!(matches!(
            set,
            ComponentDelta::Set {
                entity: changed_entity,
                component_kind: ComponentKind::Name,
                before: Some(ComponentValue::Name(Name(ref old))),
                after: ComponentValue::Name(Name(ref new))
            } if changed_entity == entity(3) && old == "Old" && new == "New"
        ));
        assert!(matches!(
            removed,
            ComponentDelta::Removed {
                entity: removed_entity,
                component_kind: ComponentKind::RewardEncumbrance,
                before: ComponentValue::RewardEncumbrance(_)
            } if removed_entity == entity(4)
        ));
    }

    #[test]
    fn relation_delta_stores_typed_semantic_payloads() {
        let relation = RelationValue::LoyalTo {
            subject: entity(6),
            target: entity(7),
            strength: Permille::new(700).unwrap(),
        };
        let added = RelationDelta::Added {
            relation_kind: RelationKind::LoyalTo,
            relation: relation.clone(),
        };
        let removed = RelationDelta::Removed {
            relation_kind: RelationKind::HostileTo,
            relation: RelationValue::HostileTo {
                subject: entity(8),
                target: entity(9),
            },
        };

        assert!(matches!(
            added,
            RelationDelta::Added {
                relation_kind: RelationKind::LoyalTo,
                relation: RelationValue::LoyalTo { strength, .. }
            } if strength == Permille::new(700).unwrap()
        ));
        assert!(matches!(
            removed,
            RelationDelta::Removed {
                relation_kind: RelationKind::HostileTo,
                relation: RelationValue::HostileTo { subject, target }
            } if subject == entity(8) && target == entity(9)
        ));
        assert_eq!(relation.kind(), RelationKind::LoyalTo);
    }

    #[test]
    fn quantity_delta_stores_before_and_after_quantities() {
        let delta = QuantityDelta::Changed {
            entity: entity(10),
            commodity: CommodityKind::Bread,
            before: Quantity(2),
            after: Quantity(5),
        };

        assert!(matches!(
            delta,
            QuantityDelta::Changed {
                entity: changed_entity,
                commodity: CommodityKind::Bread,
                before: Quantity(2),
                after: Quantity(5)
            } if changed_entity == entity(10)
        ));
    }

    #[test]
    fn reservation_delta_stores_full_reservation_record() {
        let reservation = reservation_record();
        let created = ReservationDelta::Created {
            reservation: reservation.clone(),
        };
        let released = ReservationDelta::Released {
            reservation: reservation.clone(),
        };

        assert!(matches!(
            created,
            ReservationDelta::Created { reservation: ref record } if record == &reservation
        ));
        assert!(matches!(
            released,
            ReservationDelta::Released { reservation: ref record } if record == &reservation
        ));
    }

    #[test]
    fn delta_variants_roundtrip_through_bincode() {
        let variants = [
            bincode::serialize(&EntityDelta::Created {
                entity: entity(1),
                kind: EntityKind::Agent,
            })
            .unwrap(),
            bincode::serialize(&ComponentDelta::Set {
                entity: entity(2),
                component_kind: ComponentKind::AgentData,
                before: None,
                after: ComponentValue::AgentData(AgentData {
                    control_source: ControlSource::Human,
                }),
            })
            .unwrap(),
            bincode::serialize(&RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder {
                    office: entity(3),
                    holder: entity(4),
                },
            })
            .unwrap(),
            bincode::serialize(&QuantityDelta::Changed {
                entity: entity(5),
                commodity: CommodityKind::Coin,
                before: Quantity(10),
                after: Quantity(12),
            })
            .unwrap(),
            bincode::serialize(&ReservationDelta::Released {
                reservation: reservation_record(),
            })
            .unwrap(),
        ];

        let entity_roundtrip: EntityDelta = bincode::deserialize(&variants[0]).unwrap();
        let component_roundtrip: ComponentDelta = bincode::deserialize(&variants[1]).unwrap();
        let relation_roundtrip: RelationDelta = bincode::deserialize(&variants[2]).unwrap();
        let quantity_roundtrip: QuantityDelta = bincode::deserialize(&variants[3]).unwrap();
        let reservation_roundtrip: ReservationDelta = bincode::deserialize(&variants[4]).unwrap();

        assert!(matches!(entity_roundtrip, EntityDelta::Created { .. }));
        assert!(matches!(component_roundtrip, ComponentDelta::Set { .. }));
        assert!(matches!(relation_roundtrip, RelationDelta::Added { .. }));
        assert!(matches!(quantity_roundtrip, QuantityDelta::Changed { .. }));
        assert!(matches!(
            reservation_roundtrip,
            ReservationDelta::Released { .. }
        ));
    }

    #[test]
    fn state_delta_wraps_all_delta_families() {
        let reservation = reservation_record();
        let variants = [
            StateDelta::Entity(EntityDelta::Created {
                entity: entity(1),
                kind: EntityKind::Agent,
            }),
            StateDelta::Component(ComponentDelta::Set {
                entity: entity(2),
                component_kind: ComponentKind::Name,
                before: None,
                after: ComponentValue::Name(Name("Kite".to_string())),
            }),
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::LocatedIn,
                relation: RelationValue::LocatedIn {
                    entity: entity(3),
                    place: entity(4),
                },
            }),
            StateDelta::Quantity(QuantityDelta::Changed {
                entity: entity(5),
                commodity: CommodityKind::Water,
                before: Quantity(2),
                after: Quantity(6),
            }),
            StateDelta::Reservation(ReservationDelta::Created { reservation }),
        ];

        assert!(matches!(
            variants[0],
            StateDelta::Entity(EntityDelta::Created { .. })
        ));
        assert!(matches!(
            variants[1],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            variants[2],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            variants[3],
            StateDelta::Quantity(QuantityDelta::Changed { .. })
        ));
        assert!(matches!(
            variants[4],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn state_delta_roundtrips_through_bincode() {
        let deltas = [
            StateDelta::Entity(EntityDelta::Archived {
                entity: entity(6),
                kind: EntityKind::Office,
            }),
            StateDelta::Component(ComponentDelta::Removed {
                entity: entity(7),
                component_kind: ComponentKind::Container,
                before: component_samples().pop().unwrap(),
            }),
            StateDelta::Relation(RelationDelta::Removed {
                relation_kind: RelationKind::HostileTo,
                relation: RelationValue::HostileTo {
                    subject: entity(8),
                    target: entity(11),
                },
            }),
            StateDelta::Quantity(QuantityDelta::Changed {
                entity: entity(9),
                commodity: CommodityKind::Coin,
                before: Quantity(4),
                after: Quantity(9),
            }),
            StateDelta::Reservation(ReservationDelta::Released {
                reservation: reservation_record(),
            }),
        ];

        for delta in deltas {
            let bytes = bincode::serialize(&delta).unwrap();
            let roundtrip: StateDelta = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, delta);
        }
    }

    #[test]
    fn compact_set_serialization_roundtrip() {
        let diff = ComponentDiff::BeliefStore(crate::BeliefStoreDiff::default());
        let delta = ComponentDelta::CompactSet {
            entity: entity(1),
            component_kind: ComponentKind::AgentBeliefStore,
            diff: diff.clone(),
        };

        let bytes = bincode::serialize(&delta).unwrap();
        let roundtrip: ComponentDelta = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, delta);
    }

    #[test]
    fn compact_set_in_state_delta_serialization_roundtrip() {
        let diff = ComponentDiff::BeliefStore(crate::BeliefStoreDiff::default());
        let delta = StateDelta::Component(ComponentDelta::CompactSet {
            entity: entity(1),
            component_kind: ComponentKind::AgentBeliefStore,
            diff,
        });

        let bytes = bincode::serialize(&delta).unwrap();
        let roundtrip: StateDelta = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, delta);
    }

    #[test]
    fn existing_set_serialization_unchanged_after_compact_set_addition() {
        let set_delta = ComponentDelta::Set {
            entity: entity(3),
            component_kind: ComponentKind::Name,
            before: Some(ComponentValue::Name(Name("Old".to_string()))),
            after: ComponentValue::Name(Name("New".to_string())),
        };

        // Serialize, deserialize, verify equality — confirms the Set variant's
        // bincode index is unchanged by the CompactSet addition.
        let bytes = bincode::serialize(&set_delta).unwrap();
        let roundtrip: ComponentDelta = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, set_delta);
    }
}
