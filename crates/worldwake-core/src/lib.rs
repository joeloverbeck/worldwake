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
pub mod allocator;
pub mod bandit_camp;
pub mod belief;
pub mod blocked_intent;
pub mod canonical;
pub mod cause;
pub mod cognitive_profile;
pub mod combat;
pub mod communication;
pub mod component_schema;
pub mod component_tables;
pub mod components;
pub mod conservation;
pub mod contention;
pub mod control;
pub mod crime;
pub mod delta;
pub mod drives;
pub mod entity;
pub mod entity_belief_claim;
pub mod epistemic;
pub mod expectation;
pub mod error;
pub mod event_log;
pub mod event_record;
pub mod event_tag;
pub mod evidence;
pub mod execution_budget;
pub mod experience;
pub mod factions;
pub mod goal;
pub mod ids;
pub mod institutional;
pub mod intention;
pub mod intention_disposition;
pub mod intention_frame;
pub mod items;
pub mod load;
pub mod needs;
pub mod numerics;
pub mod offices;
pub mod observation_context;
pub mod patrol;
pub mod production;
pub mod pursuit;
pub mod relations;
pub mod rights;
pub mod social_artifact;
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
pub use allocator::EntityAllocator;
pub use bandit_camp::{BanditCamp, BanditFactionPolicy};
pub use belief::{
    AgentBeliefStore, AskWitnessMemory, AskWitnessMemoryKey, BeliefConfidencePolicy,
    BelievedActivity, BelievedArtifactState, BelievedBountyTerms, BelievedContentionState,
    BelievedEntityState, BelievedEvidenceEntry, BelievedEvidenceState, HeardBeliefDisposition,
    HeardBeliefMemory, MismatchKind, ObservedEntitySnapshot, PerceptionProfile, PerceptionSource,
    RecipientKnowledgeStatus, SharedBeliefSnapshot, SharedInstitutionalBelief, SharedTellState,
    SocialObservation, SocialObservationDetail, SocialObservationKind, TellMemoryKey, TellProfile,
    TellTopic, ToldBeliefMemory, belief_confidence, build_believed_entity_state,
    build_observed_entity_snapshot, current_institutional_belief_topics,
    institutional_claim_same_memory_lane, institutional_claim_subject_entity,
    institutional_knowledge_chain_len, recipient_knowledge_status, share_equivalent,
    social_observation_is_redundant_for_listener, social_observation_is_relayable,
    tell_subject_is_directly_observable_by_listener, to_shared_belief_snapshot,
};
pub use blocked_intent::{
    BlockedIntent, BlockedIntentMemory, BlockerClearingCondition, BlockerDiagnostic, BlockerKey,
    BlockingFact, ClearingBaseline,
};
pub use canonical::{
    CanonicalError, StateHash, canonical_bytes, hash_bytes, hash_event_log, hash_serializable,
    hash_world,
};
pub use cause::CauseRef;
pub use cognitive_profile::CognitiveProfile;
pub use combat::{CombatProfile, CombatStance, DeadAt};
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
};
pub use control::ControlSource;
pub use crime::{
    JusticeDispositionProfile, PunishmentFineSelectionTrace, PunishmentFineStartFailureTrace,
    PunishmentFineTraceFacts, PunishmentKind, TheftDispositionProfile, TheftFacts,
};
pub use delta::{
    ComponentDelta, ComponentKind, ComponentValue, EntityDelta, QuantityDelta, RelationDelta,
    RelationKind, RelationValue, ReservationDelta, StateDelta,
};
pub use drives::{DriveThresholds, ThresholdBand};
pub use entity::{EntityKind, EntityMeta};
pub use entity_belief_claim::{ClaimId, ClaimValue, EntityBeliefAspect, EntityBeliefClaim};
pub use epistemic::{EpistemicDispositionProfile, EpistemicSubject};
pub use expectation::{
    ExpectationBasis, ExpectationId, ExpectationOutcome, ExpectationRecord, ExpectationState,
    LastSeenProvenance, LastSeenRecord, SearchCondition, SearchResult, SearchTarget,
};
pub use error::WorldError;
pub use event_log::EventLog;
pub use event_record::{EventPayload, EventRecord, EventView, EvidenceRef, PendingEvent};
pub use event_tag::EventTag;
pub use evidence::{DisturbanceKind, EvidenceEntry, EvidenceEntryId, EvidenceKind, SceneEvidence};
pub use execution_budget::ExecutionBudget;
pub use experience::{
    EdgeExperience, PreferenceProfile, ReliabilityRecord, RouteExperience, SourceKey,
    SourceReliability, danger_ratio_permille, failure_ratio_permille,
};
pub use factions::{FactionData, FactionPurpose};
pub use goal::{CommodityPurpose, GoalKey, GoalKind, OpportunityAnchor, OpportunityKey};
pub use ids::{ActionDefId, EntityId, EventId, ReservationId, Seed, Tick, TickRange, TravelEdgeId};
pub use institutional::{
    BelievedInstitutionalClaim, InstitutionalBeliefKey, InstitutionalBeliefRead,
    InstitutionalClaim, InstitutionalKnowledgeSource, InstitutionalRecordEntry,
    InstitutionalRecordError, RecordData, RecordEntryId, RecordKind,
};
pub use intention::ActiveGoal;
pub use intention_disposition::IntentionDispositionProfile;
pub use intention_frame::{
    FrameAssumption, FrameClearReason, FrameState, IntentionDomain, IntentionDomainTag,
    IntentionFrame, SuspensionReason,
};
pub use items::{
    CombatWeaponProfile, CommodityConsumableProfile, CommodityKind, CommodityKindSpec,
    CommodityPhysicalProfile, CommodityTreatmentProfile, Container, ItemLot, LotOperation,
    ProvenanceEntry, TradeCategory, UniqueItem, UniqueItemKind, UniqueItemKindSpec,
    UniqueItemPhysicalProfile,
};
pub use load::{
    current_container_load, load_of_entity, load_of_lot, load_of_unique_item,
    load_of_unique_item_kind, load_per_unit, remaining_container_capacity,
};
pub use needs::{
    BodyCostPerTick, DeprivationExposure, HomeostaticNeedId, HomeostaticNeeds, MetabolismProfile,
};
pub use numerics::{LoadUnits, Permille, Quantity};
pub use offices::{
    EligibilityRule, OfficeData, OfficeForceProfile, OfficeForceState, SuccessionLaw,
};
pub use observation_context::{ObservationContext, PlaceVisibilityProfile};
pub use patrol::{PatrolProfile, PatrolRoute};
pub use production::{
    CarryCapacity, InTransitOnEdge, KnownRecipes, ProductionJob, ProductionOutputOwner,
    ProductionOutputOwnershipPolicy, RecipeId, ResourceSource, WorkstationMarker, WorkstationTag,
};
pub use pursuit::PursuitProfile;
pub use relations::{ArchiveDependency, ArchiveDependencyKind, RelationTables, ReservationRecord};
pub use rights::{EffectiveRight, RightKind};
pub use social_artifact::{
    ArtifactHeader, ArtifactKind, ArtifactPostingContext, ArtifactState, BountyTarget, BountyTerms,
    NoticeContent, NoticeTopic, ProofRequirement, RewardSource,
};
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
