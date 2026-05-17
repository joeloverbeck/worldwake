//! # worldwake-core
//!
//! Foundation crate for the Worldwake simulation: IDs, types, ECS store,
//! topology, items, and relations. Has no internal crate dependencies.
//!
//! ## Deterministic Data Policy
//!
//! All authoritative simulation state **must** use only deterministic,
//! serializable data structures:
//!
//! **Allowed** in authoritative state:
//! - `Vec`, `Option`, `BTreeMap`, `BTreeSet`
//! - Fixed-width integers (`u8`..`u128`, `i8`..`i128`)
//! - Enums / structs composed of the above
//!
//! **Forbidden** in authoritative or hashed state:
//! - `HashMap`, `HashSet` (non-deterministic iteration order)
//! - `TypeId`, `Box<dyn Any>` (opaque, not serializable)
//! - Raw pointer identity
//! - Wall-clock time
//! - Floating-point values unless there is a written exception and a
//!   canonicalization rule
//!
//! This policy is enforced by integration tests that scan source files for
//! forbidden patterns.

pub mod action_domain;
pub mod agenda_profile;
pub mod allocator;
pub mod bandit_camp;
pub mod belief;
pub mod belief_claim_key;
pub mod blocker_memory;
pub mod blocker_scope;
pub mod canonical;
pub mod causal_link;
pub mod cause;
pub mod cognitive_profile;
pub mod combat;
pub mod communication;
pub mod component_schema;
pub mod component_tables;
pub mod components;
pub mod conservation;
pub mod contention;
pub mod contention_event;
pub mod control;
pub mod crime;
pub mod debug_view;
pub mod decision_event_payload;
pub mod delta;
pub mod discrepancy;
pub mod disposal;
pub mod diversification;
pub mod drive_escalation_profile;
pub mod drives;
pub mod entity;
pub mod entity_belief_claim;
pub mod epistemic;
pub mod error;
pub mod event_log;
pub mod event_record;
pub mod event_tag;
pub mod evidence;
pub mod execution_budget;
pub mod expectation;
pub mod experience;
pub mod exploration;
pub mod factions;
pub mod goal;
pub mod ids;
pub mod institutional;
pub mod intention_disposition;
pub mod intention_frame;
pub mod items;
pub mod law_abiding_profile;
pub mod learned_opportunity_memory;
pub mod load;
pub mod materialization_tag;
pub mod memory_capacity_profile;
pub mod motive_source;
pub mod needs;
pub mod numerics;
pub mod obligation;
pub mod observation_context;
pub mod offices;
pub mod patrol;
pub mod percentile;
pub mod place_dirtiness;
pub mod plan_step_guards;
pub mod production;
pub mod pursuit;
pub mod relations;
pub mod repair_memory;
pub mod reward_encumbrance;
pub mod rights;
pub mod risk_weight_profile;
pub mod sleep_episode;
pub mod social_artifact;
pub mod survey_memory;
pub mod test_utils;
pub mod topology;
pub mod trade;
pub mod traits;
pub mod utility_profile;
pub mod valuation;
pub mod verification;
pub mod violation;
pub mod visibility;
pub mod witness;
pub mod world;
pub mod world_txn;
pub mod wounds;

pub use action_domain::ActionDomain;
pub use agenda_profile::AgendaProfile;
pub use allocator::EntityAllocator;
pub use bandit_camp::{BanditCamp, BanditFactionPolicy};
pub use belief::{
    AgentBeliefStore, AskWitnessMemory, AskWitnessMemoryKey, BeliefConfidencePolicy,
    BeliefStoreDiff, BelievedActivity, BelievedArtifactState, BelievedBountyTerms,
    BelievedContentionState, BelievedEntityState, BelievedEvidenceEntry, BelievedEvidenceState,
    HeardBeliefDisposition, HeardBeliefMemory, MismatchKind, ObservationOmission,
    ObservationOmissionLog, ObservedEntitySnapshot, OmissionReason, PerceptionProfile,
    PerceptionSource, PlaceVisitRecord, RecipientKnowledgeStatus, SaliencePolicy,
    SharedBeliefSnapshot, SharedInstitutionalBelief, SharedTellState, SocialObservation,
    SocialObservationDetail, SocialObservationKind, TellMemoryKey, TellProfile, TellTopic,
    ToldBeliefMemory, belief_confidence, build_believed_entity_state,
    build_observed_entity_snapshot, current_institutional_belief_topics,
    default_omission_log_capacity, default_opportunity_floor_permille, effective_claim_confidence,
    institutional_claim_same_memory_lane, institutional_claim_subject_entity,
    institutional_knowledge_chain_len, recipient_knowledge_status, share_equivalent,
    social_observation_is_redundant_for_listener, social_observation_is_relayable,
    tell_subject_is_directly_observable_by_listener, to_shared_belief_snapshot,
};
pub use belief_claim_key::BeliefClaimKey;
pub use blocker_memory::{
    Blocker, BlockerClearingCondition, BlockerDiagnostic, BlockerKey, BlockerMemory, BlockingFact,
    ClearingBaseline,
};
pub use blocker_scope::{BlockerScope, RouteSegment};
pub use canonical::{
    CanonicalError, StateHash, canonical_bytes, hash_bytes, hash_event_log, hash_serializable,
    hash_world,
};
pub use causal_link::{CausalLink, CausalProvider, PlanningFact, RecordTopic};
pub use cause::CauseRef;
pub use cognitive_profile::{CognitiveProfile, PortfolioSlotWeights};
pub use combat::{CombatProfile, CombatStance, DeadAt, DeathCause};
pub use communication::{CommunicationClass, CommunicationProfile, classify_communication};
pub use component_tables::ComponentTables;
pub use components::{AgentData, Name};
pub use conservation::{
    total_authoritative_commodity_quantity, total_live_lot_quantity,
    verify_authoritative_conservation, verify_live_lot_conservation,
};
pub use contention::{
    ContentionDispositionProfile, ContentionError, ContentionGrant, ContentionIntents,
    ContentionPolicy, ContentionQueue, ContentionStatus, ContentionWaiter, QueuedContentionIntent,
    ResourceExtractionQueues,
};
pub use contention_event::{
    AffordanceKey, ClaimantOutcome, ContentionClaimant, ContentionEventPayload,
    ContentionResolutionRule, DenialReason, build_contention_event_payload,
};
pub use control::ControlSource;
pub use crime::{
    JusticeDispositionProfile, PunishmentFineSelectionTrace, PunishmentFineStartFailureTrace,
    PunishmentFineTraceFacts, PunishmentKind, TheftDispositionProfile, TheftFacts,
};
pub use debug_view::EntityState;
pub use decision_event_payload::{
    ActionInterruptReasonTag, BeliefRef, BeliefSnapshot, BeliefStatusTag, BlockerRecordedPayload,
    DecisionEventPayload, EmitterTag, EvidenceKindTag, EvidenceSummary, ExpectationFailureCauseTag,
    ExpectationFailurePhaseTag, ExpectationMismatchPayload, GoalAbandonReason,
    GoalAbandonedPayload, GoalCommittedPayload, GoalOfferedPayload, GoalRejectionReason,
    GoalSuppressedPayload, GoalSuspendedPayload, GoalSwitchReason, ObservationRef,
    OpportunityExpectationKindTag, PlanAdoptedPayload, PlanAssumptionRef, PlanInvalidatedPayload,
    PlanInvalidationReason, PursuitInvalidationReasonTag, RankedGoalComparisonDimensionTag,
    RecordRef, RejectedAlternativeSummary, RepairAppliedPayload, RepairKind, ReplanReason,
    ReplanTriggeredPayload, SleepEpisodeEndedPayload, SleepEpisodeStartedPayload,
    SourceAttributionOutcomeTag, SourceExpectationFailurePayload, SourceKeyPayload,
    SurveyRecordedPayload, WakeReason, WashFacilityUsedPayload, WasteCreatedPayload, WasteSource,
};
pub use delta::{
    ComponentDelta, ComponentDiff, ComponentKind, ComponentValue, EntityDelta, QuantityDelta,
    RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta,
};
pub use discrepancy::{Discrepancy, DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory};
pub use disposal::DisposalProfile;
pub use diversification::{DiversificationProfile, LastProactiveExplorationTick};
pub use drive_escalation_profile::{
    DriveEscalationParams, DriveEscalationProfile, MultiplierPermille, escalation_multiplier,
};
pub use drives::{DriveThresholds, ThresholdBand};
pub use entity::{EntityKind, EntityMeta};
pub use entity_belief_claim::{ClaimId, ClaimValue, EntityBeliefAspect, EntityBeliefClaim};
pub use epistemic::{EpistemicDispositionProfile, EpistemicSubject};
pub use error::{ControlDeniedReason, WorldError};
pub use event_log::{CheckpointData, EventLog};
pub use event_record::{EventPayload, EventRecord, EventView, EvidenceRef, PendingEvent};
pub use event_tag::EventTag;
pub use evidence::{DisturbanceKind, EvidenceEntry, EvidenceEntryId, EvidenceKind, SceneEvidence};
pub use execution_budget::ExecutionBudget;
pub use expectation::{
    ExpectationBasis, ExpectationId, ExpectationOutcome, ExpectationRecord, ExpectationState,
    ExpectationStore, LastSeenMemory, LastSeenProvenance, LastSeenRecord, SearchCondition,
    SearchResult,
};
pub use experience::{
    EdgeExperience, PreferenceProfile, ReliabilityRecord, RouteExperience, SourceKey,
    SourceReliability, danger_ratio_permille, failure_ratio_permille,
};
pub use exploration::{AcquisitionExhaustionTracker, ExplorationProfile};
pub use factions::{FactionData, FactionPurpose};
pub use goal::{
    AcquisitionQuantity, CommodityPurpose, ExplorationMotivation, GoalKey, GoalKind,
    HypothesisKind, OpportunityAnchor, OpportunityKey,
};
pub use ids::{ActionDefId, EntityId, EventId, ReservationId, Seed, Tick, TickRange, TravelEdgeId};
pub use institutional::{
    BelievedInstitutionalClaim, InstitutionalBeliefKey, InstitutionalBeliefRead,
    InstitutionalClaim, InstitutionalKnowledgeSource, InstitutionalRecordEntry,
    InstitutionalRecordError, RecordData, RecordEntryId, RecordKind,
};
pub use intention_disposition::IntentionDispositionProfile;
pub use intention_frame::{
    FrameAssumption, FrameClearReason, FrameState, IntentionDomain, IntentionDomainTag,
    IntentionFrame, SuspensionReason,
};
pub use items::{
    CombatWeaponProfile, CommodityConsumableProfile, CommodityDecayMap, CommodityKind,
    CommodityKindSpec, CommodityPhysicalProfile, CommodityTreatmentProfile, Container, GroundSince,
    ItemLot, LotOperation, ProvenanceEntry, TradeCategory, UniqueItem, UniqueItemKind,
    UniqueItemKindSpec, UniqueItemPhysicalProfile, default_commodity_decay_map,
};
pub use law_abiding_profile::LawAbidingProfile;
pub use learned_opportunity_memory::{LearnedOpportunityMemory, OpportunityEntry};
pub use load::{
    current_container_load, load_of_entity, load_of_lot, load_of_unique_item,
    load_of_unique_item_kind, load_per_unit, remaining_container_capacity,
};
pub use materialization_tag::MaterializationTag;
pub use memory_capacity_profile::MemoryCapacityProfile;
pub use motive_source::{MotiveSource, MotiveSourceRef};
pub use needs::{
    BodyCostPerTick, DeprivationExposure, HomeostaticNeedId, HomeostaticNeeds, MetabolismProfile,
};
pub use numerics::{LoadUnits, Permille, Quantity};
pub use obligation::{ObligationExecutionTracker, ObligationSatiationProfile};
pub use observation_context::{ObservationContext, PlaceVisibilityProfile};
pub use offices::{
    EligibilityRule, OfficeData, OfficeForceProfile, OfficeForceState, SuccessionLaw,
};
pub use patrol::{PatrolProfile, PatrolRoute};
pub use percentile::PercentileBucket;
pub use place_dirtiness::{LatrineFullness, PlaceDirtiness, WashBasinState};
pub use plan_step_guards::{
    ExpectationKindTag, InvalidatorTag, MismatchDetail, ObservationPredicate, StatePredicate,
};
pub use production::{
    CarryCapacity, HARVEST_TRACE_MAX_ENTRIES, HARVEST_TRACE_RETENTION_TICKS, HarvestTraceEntry,
    InTransitOnEdge, KnownRecipes, LastHarvestTrace, ProductionJob, ProductionOutputOwner,
    ProductionOutputOwnershipPolicy, RecipeId, ResourceSource, WorkstationMarker, WorkstationTag,
};
pub use pursuit::PursuitProfile;
pub use relations::{ArchiveDependency, ArchiveDependencyKind, RelationTables, ReservationRecord};
pub use repair_memory::{BreachSignature, RepairEntry, RepairMemory};
pub use reward_encumbrance::{RewardEncumbrance, RewardReservation};
pub use rights::{EffectiveRight, RightKind};
pub use risk_weight_profile::RiskWeightProfile;
pub use sleep_episode::{
    GroundComfortTag, ShelterTag, SleepEpisode, SleepQualityProfile, SleepRecoveryModifier,
    WakeCondition,
};
pub use social_artifact::{
    ArtifactActionability, ArtifactAxisValue, ArtifactCredibility, ArtifactExistence,
    ArtifactHeader, ArtifactKind, ArtifactLegalEffect, ArtifactPostingContext,
    ArtifactPostingProfile, ArtifactTransitionPayload, ArtifactVisibility, AxisName, BlockerReason,
    BountyTarget, BountyTerms, CloseCause, DestructionCause, NoticeContent, NoticeTopic, ProofKind,
    ProofRequirement, RevocationReason, RewardSource,
};
pub use survey_memory::{SurveyMemory, SurveyRecord};
pub use topology::{
    OUTDOOR_RELIEF_TAGS, Place, PlaceTag, PlaceTagSet, PrototypePlace, Route, Topology, TravelEdge,
    build_prototype_world, prototype_place_entity,
};
pub use trade::{
    DemandMemory, DemandObservation, DemandObservationReason, MerchandiseProfile, SaleListing,
    StockAssignment, StockAssignmentKind, StockStoragePolicy, SubstitutePreferences,
    TradeDispositionProfile, TradeRole,
};
pub use traits::{Component, RelationRecord};
pub use utility_profile::UtilityProfile;
pub use valuation::CommodityValuationProfile;
pub use verification::{VerificationError, verify_completeness};
pub use violation::{
    RecordedViolation, ViolationDispositionProfile, ViolationId, ViolationKind, ViolationMemory,
};
pub use visibility::VisibilitySpec;
pub use witness::WitnessData;
pub use world::World;
pub use world::lifecycle::{
    ArchiveMutationSnapshot, ArchivePreparationAction, ArchivePreparationPlan,
    ArchivePreparationPolicy, ArchivePreparationReport, ArchiveResolution,
};
pub use world_txn::WorldTxn;
pub use wounds::{
    BodyPart, CombatWeaponRef, DeprivationKind, Wound, WoundCause, WoundId, WoundList,
    is_incapacitated, is_wound_load_fatal,
};
