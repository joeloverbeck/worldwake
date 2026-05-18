//! Scenario definition types for RON-based world initialization.
//!
//! These are pure data types — no logic, just the schema for scenario files.
//! All location references use string names, resolved to `EntityId` during spawning.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::Deserialize;
use worldwake_core::{
    AgendaProfile, AgentSchemaContextProfile, ArtifactCredibility, ArtifactExistence,
    ArtifactLegalEffect, ArtifactPostingProfile, BlockerReason, CarryCapacity, CloseCause,
    CognitiveProfile, CombatProfile, CommodityDecayMap, CommodityValuationProfile,
    CommunicationProfile, Container, ContentionDispositionProfile, ContentionPolicy, ControlSource,
    DestructionCause, DisposalProfile, DiversificationProfile, DriveEscalationProfile,
    DriveThresholds, EpistemicDispositionProfile, ExecutionBudget, ExplorationProfile,
    GroundComfortTag, HomeostaticNeeds, IntentionDispositionProfile, JusticeDispositionProfile,
    LatrineFullness, LawAbidingProfile, LoadUnits, MetabolismProfile, ObligationSatiationProfile,
    PatrolProfile, PerceptionProfile, PerceptionSource, Permille, PlaceDirtiness,
    PlaceVisibilityProfile, PortfolioWeightsProfile, PreferenceProfile, ProofKind,
    ProofRequirement, PursuitProfile, Quantity, RevocationReason, RiskWeightProfile,
    RoutePreferenceProfile, ShelterTag, SleepQualityProfile, SleepRecoveryModifier,
    SubstitutePreferences, SuccessionLaw, TellProfile, TestimonyTrustProfile,
    TheftDispositionProfile, TradeDispositionProfile, UtilityProfile, ViolationDispositionProfile,
    WashBasinState, WorkstationTag, items::CommodityKind,
    social_artifact::SuspensionReason as ArtifactSuspensionReason, topology::PlaceTag,
};

/// Top-level scenario definition. Describes an entire world to initialize.
#[derive(Clone, Debug, Deserialize)]
pub struct ScenarioDef {
    pub seed: u64,
    pub places: Vec<PlaceDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
    #[serde(default)]
    pub bandit_camps: Vec<BanditCampDef>,
    #[serde(default)]
    pub offices: Vec<OfficeDef>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub facilities: Vec<FacilityDef>,
    #[serde(default)]
    pub resource_sources: Vec<ResourceSourceDef>,
    #[serde(default)]
    pub hostilities: Vec<HostilityDef>,
    #[serde(default)]
    pub commodity_decay: Option<CommodityDecayMap>,
    #[serde(default)]
    pub survival_health_contract: Option<SurvivalHealthContractDef>,
    /// Ticks between checkpoint snapshots for event log compaction.
    /// Default: 50. Set to 0 to disable compaction.
    #[serde(default = "default_compaction_interval")]
    pub compaction_interval: u32,
    /// Per-rule lint overrides keyed by `LintRule` variant.
    /// Values must be non-empty justification strings.
    #[serde(default)]
    pub scenario_lint_overrides: BTreeMap<crate::scenario::lints::LintRule, String>,
    /// Optional override for the `LastHarvestTrace` retention window
    /// (ticks). When `None`, the global default
    /// `worldwake_core::HARVEST_TRACE_RETENTION_TICKS` is used. (S127 §D5/§D10)
    #[serde(default)]
    pub harvest_trace_retention_ticks: Option<u32>,
}

/// Authored directed hostility relation between two named entities.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostilityDef {
    pub subject: String,
    pub target: String,
}

/// An authored active bandit camp and its owning faction policy.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanditCampDef {
    pub faction: String,
    pub place: String,
    pub members: Vec<String>,
    pub policy: BanditFactionPolicyDef,
    #[serde(default)]
    pub supplies: Option<BanditCampSuppliesDef>,
}

/// Scenario-specific bandit faction policy using string place references.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanditFactionPolicyDef {
    pub min_regroup_count: u8,
    pub establishment_duration_ticks: NonZeroU32,
    pub abandonment_grace_ticks: NonZeroU32,
    pub flee_wound_threshold: Permille,
    #[serde(default)]
    pub rally_place: Option<String>,
}

/// Optional initial camp supplies.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanditCampSuppliesDef {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub container: Container,
}

/// Authored initial funds held by an office-owned treasury container.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreasuryDef {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    #[serde(default)]
    pub container_name: Option<String>,
}

/// An authored office entity plus its automatically maintained local records.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeDef {
    pub name: String,
    pub seat: String,
    pub succession_law: SuccessionLaw,
    pub succession_period_ticks: u64,
    #[serde(default)]
    pub initial_holder: Option<String>,
    #[serde(default)]
    pub eligibility_rules: Vec<EligibilityRuleDef>,
    #[serde(default)]
    pub treasury: Option<TreasuryDef>,
}

/// Scenario-specific eligibility rule using string references instead of `EntityId`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EligibilityRuleDef {
    FactionMember(String),
}

/// An authored social artifact using string references instead of `EntityId`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDef {
    pub kind: ArtifactKindDef,
    pub issuer: String,
    pub location: String,
    #[serde(default)]
    pub issuing_authority: Option<String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    pub payload: ArtifactPayloadDef,
    #[serde(default)]
    pub existence: Option<ArtifactExistenceDef>,
    #[serde(default)]
    pub visibility: Option<ArtifactVisibilityDef>,
    #[serde(default)]
    pub legal_effect: Option<ArtifactLegalEffectDef>,
    #[serde(default)]
    pub credibility: Option<ArtifactCredibilityDef>,
    #[serde(default)]
    pub actionability: Option<ArtifactActionabilityDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactKindDef {
    Notice,
    Bounty,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactPayloadDef {
    Notice(NoticeTopicDef),
    Bounty(BountyTermsDef),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum NoticeTopicDef {
    ThreatWarning {
        place: String,
    },
    OfficeVacancy {
        office: String,
    },
    CommodityShortage {
        commodity: CommodityKind,
        place: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BountyTermsDef {
    pub target: BountyTargetDef,
    pub proof_requirement: ProofRequirement,
    pub reward_commodity: CommodityKind,
    pub reward_quantity: Quantity,
    pub reward_source: RewardSourceDef,
    pub claim_place: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum BountyTargetDef {
    EliminateEntity {
        target: String,
    },
    DeliverCommodity {
        commodity: CommodityKind,
        quantity: Quantity,
        destination: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RewardSourceDef {
    InstitutionalTreasury { treasury_entity: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactExistenceDef {
    Exists,
    Destroyed {
        destroyed_at: u64,
        cause: DestructionCause,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactVisibilityDef {
    Hidden,
    Private { audience: Vec<String> },
    Posted { place: String },
    WidelyKnown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactLegalEffectDef {
    None,
    Active {
        #[serde(default)]
        expires_at: Option<u64>,
    },
    Suspended {
        reason: ArtifactSuspensionReason,
        suspended_at: u64,
    },
    Expired {
        expired_at: u64,
    },
    Revoked {
        revoked_at: u64,
        by: String,
        reason: RevocationReason,
    },
    Fulfilled {
        fulfilled_at: u64,
        by: String,
        evidence: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactCredibilityDef {
    Credible,
    Disputed {
        disputed_at: u64,
        contradicting: Vec<String>,
    },
    Refuted {
        refuted_at: u64,
        evidence: String,
    },
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ArtifactActionabilityDef {
    Actionable,
    AwaitingProof { required_proof: ProofKind },
    Blocked { reason: BlockerReason, since: u64 },
    Closed { closed_at: u64, cause: CloseCause },
}

pub fn default_artifact_existence() -> ArtifactExistence {
    ArtifactExistence::Exists
}

pub fn default_artifact_legal_effect(expires_at: Option<u64>) -> ArtifactLegalEffect {
    ArtifactLegalEffect::Active {
        expires_at: expires_at.map(worldwake_core::Tick),
    }
}

pub fn default_artifact_credibility() -> ArtifactCredibility {
    ArtifactCredibility::Credible
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SocialObservationDef {
    pub place: String,
    pub observed_tick: u64,
    pub source: PerceptionSource,
    pub detail: SocialObservationDetailDef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum SocialObservationDetailDef {
    WitnessedConflict { actor: String, target: String },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpectationStoreDef {
    #[serde(default)]
    pub records: Vec<ExpectationRecordDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpectationRecordDef {
    pub subject: String,
    pub expected_place: String,
    pub deadline_tick: u64,
    pub grace_ticks: u64,
    pub basis: ExpectationBasisDef,
    pub state: ExpectationStateDef,
    pub created_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ExpectationBasisDef {
    DutyAssignment {
        office: String,
    },
    DeliveryCommitment {
        commodity: CommodityKind,
        quantity: Quantity,
    },
    RoutineReturn,
    EscortObligation {
        charge: String,
    },
    SocialPromise,
    PlanStepCompletion {
        step_index: u16,
        kind_tag: worldwake_core::ExpectationKindTag,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ExpectationStateDef {
    Active,
    Overdue,
    Resolved { outcome: ExpectationOutcomeDef },
    Expired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ExpectationOutcomeDef {
    Fulfilled,
    FoundSafe { at_place: String },
    FoundWounded { at_place: String },
    FoundDead { at_place: String },
    NotFound,
    ReturnedLate,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LastSeenMemoryDef {
    #[serde(default)]
    pub records: Vec<LastSeenRecordDef>,
    #[serde(default = "default_last_seen_capacity")]
    pub capacity: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LastSeenRecordDef {
    pub subject: String,
    pub place: String,
    pub observed_tick: u64,
    pub source: String,
    pub provenance: LastSeenProvenanceDef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum LastSeenProvenanceDef {
    DirectObservation,
    Hearsay {
        original_observer: String,
        chain_depth: u8,
    },
}

fn default_compaction_interval() -> u32 {
    50
}

const fn default_last_seen_capacity() -> u16 {
    20
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SurvivalHealthContractDef {
    pub max_authored_critical_run_ticks: u32,
    pub max_idle_window_ticks_with_elevated_need: u32,
    pub elevated_need_floor: Permille,
    pub required_self_care_families: Vec<NeedsActionFamily>,
    #[serde(default)]
    pub critical_run_limits: Option<SurvivalCriticalRunLimitsDef>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SurvivalCriticalRunLimitsDef {
    #[serde(default)]
    pub hunger: Option<u32>,
    #[serde(default)]
    pub thirst: Option<u32>,
    #[serde(default)]
    pub fatigue: Option<u32>,
    #[serde(default)]
    pub bladder: Option<u32>,
    #[serde(default)]
    pub dirtiness: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum NeedsActionFamily {
    Eat,
    Drink,
    Sleep,
    Relieve,
    Wash,
}

/// A place in the world graph.
#[derive(Clone, Debug, Deserialize)]
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
    #[serde(default)]
    pub sleep_quality: Option<SleepQualityProfileDef>,
    #[serde(default)]
    pub place_dirtiness: Option<PlaceDirtinessDef>,
    #[serde(default)]
    pub latrine_fullness: Option<LatrineFullnessDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SleepQualityProfileDef {
    pub shelter: ShelterTag,
    pub ground_comfort: GroundComfortTag,
    pub recovery_modifier: u16,
}

impl SleepQualityProfileDef {
    pub fn into_profile(self) -> Result<SleepQualityProfile, String> {
        Ok(SleepQualityProfile {
            shelter: self.shelter,
            ground_comfort: self.ground_comfort,
            recovery_modifier: SleepRecoveryModifier::new(self.recovery_modifier),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct PlaceDirtinessDef {
    pub value: Option<Permille>,
    pub decay_per_tick: Option<Permille>,
    pub dirtiness_per_use: Option<Permille>,
}

impl From<PlaceDirtinessDef> for PlaceDirtiness {
    fn from(def: PlaceDirtinessDef) -> Self {
        let defaults = PlaceDirtiness::default();
        Self {
            value: def.value.unwrap_or(defaults.value),
            decay_per_tick: def.decay_per_tick.unwrap_or(defaults.decay_per_tick),
            dirtiness_per_use: def.dirtiness_per_use.unwrap_or(defaults.dirtiness_per_use),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LatrineFullnessDef {
    pub fill: Option<Permille>,
    pub fill_per_use: Option<Permille>,
    pub critical_threshold: Option<Permille>,
}

impl From<LatrineFullnessDef> for LatrineFullness {
    fn from(def: LatrineFullnessDef) -> Self {
        let defaults = LatrineFullness::default();
        Self {
            fill: def.fill.unwrap_or(defaults.fill),
            fill_per_use: def.fill_per_use.unwrap_or(defaults.fill_per_use),
            critical_threshold: def
                .critical_threshold
                .unwrap_or(defaults.critical_threshold),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct WashBasinStateDef {
    pub clean_water_units: Option<u16>,
    pub max_clean_water: Option<u16>,
    pub refill_per_tick: Option<u16>,
    pub units_per_full_wash: Option<u16>,
    pub dirtiness_level: Option<Permille>,
    pub dirtiness_per_use: Option<Permille>,
}

impl From<WashBasinStateDef> for WashBasinState {
    fn from(def: WashBasinStateDef) -> Self {
        let defaults = WashBasinState::default();
        Self {
            clean_water_units: def.clean_water_units.unwrap_or(defaults.clean_water_units),
            max_clean_water: def.max_clean_water.unwrap_or(defaults.max_clean_water),
            refill_per_tick: def.refill_per_tick.unwrap_or(defaults.refill_per_tick),
            units_per_full_wash: def
                .units_per_full_wash
                .unwrap_or(defaults.units_per_full_wash),
            dirtiness_level: def.dirtiness_level.unwrap_or(defaults.dirtiness_level),
            dirtiness_per_use: def.dirtiness_per_use.unwrap_or(defaults.dirtiness_per_use),
        }
    }
}

/// A travel edge connecting two places.
#[derive(Clone, Debug, Deserialize)]
pub struct EdgeDef {
    pub from: String,
    pub to: String,
    pub travel_ticks: u32,
    #[serde(default = "default_true")]
    pub bidirectional: bool,
}

/// An agent to spawn in the world.
#[derive(Clone, Debug, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub location: String,
    pub control: ControlSource,
    #[serde(default)]
    pub needs: Option<HomeostaticNeeds>,
    #[serde(default)]
    pub combat_profile: Option<CombatProfile>,
    #[serde(default)]
    pub utility_profile: Option<UtilityProfile>,
    #[serde(default)]
    pub artifact_posting_profile: Option<ArtifactPostingProfile>,
    #[serde(default)]
    pub merchandise_profile: Option<MerchandiseProfileDef>,
    #[serde(default)]
    pub trade_disposition: Option<TradeDispositionProfile>,
    #[serde(default)]
    pub perception_profile: Option<PerceptionProfile>,
    #[serde(default)]
    pub tell_profile: Option<TellProfile>,
    #[serde(default)]
    pub cognitive_profile: Option<CognitiveProfile>,
    #[serde(default)]
    pub portfolio_weights_profile: Option<PortfolioWeightsProfile>,
    #[serde(default)]
    pub agent_schema_context_profile: Option<AgentSchemaContextProfile>,
    #[serde(default)]
    pub risk_weight_profile: Option<RiskWeightProfile>,
    #[serde(default)]
    pub law_abiding_profile: Option<LawAbidingProfile>,
    #[serde(default)]
    pub agenda_profile: Option<AgendaProfile>,
    #[serde(default)]
    pub execution_budget: Option<ExecutionBudget>,
    #[serde(default)]
    pub epistemic_disposition: Option<EpistemicDispositionProfile>,
    #[serde(default)]
    pub intention_disposition: Option<IntentionDispositionProfile>,
    #[serde(default)]
    pub communication_profile: Option<CommunicationProfile>,
    #[serde(default)]
    pub preference_profile: Option<PreferenceProfile>,
    #[serde(default)]
    pub expectation_store: Option<ExpectationStoreDef>,
    #[serde(default)]
    pub last_seen_memory: Option<LastSeenMemoryDef>,
    #[serde(default)]
    pub social_observations: Option<Vec<SocialObservationDef>>,
    #[serde(default)]
    pub obligation_satiation_profile: Option<ObligationSatiationProfile>,
    #[serde(default)]
    pub drive_thresholds: Option<DriveThresholds>,
    #[serde(default)]
    pub drive_escalation_profile: Option<DriveEscalationProfile>,
    #[serde(default)]
    pub metabolism_profile: Option<MetabolismProfile>,
    #[serde(default)]
    pub disposal_profile: Option<DisposalProfile>,
    #[serde(default)]
    pub exploration_profile: Option<ExplorationProfileDef>,
    #[serde(default)]
    pub diversification_profile: Option<DiversificationProfile>,
    #[serde(default)]
    pub carry_capacity: Option<CarryCapacity>,
    #[serde(default)]
    pub theft_disposition: Option<TheftDispositionProfile>,
    #[serde(default)]
    pub justice_disposition: Option<JusticeDispositionProfile>,
    #[serde(default)]
    pub violation_disposition: Option<ViolationDispositionProfile>,
    #[serde(default)]
    pub patrol_profile: Option<PatrolProfile>,
    #[serde(default)]
    pub patrol_route: Option<PatrolRouteDef>,
    #[serde(default)]
    pub pursuit_profile: Option<PursuitProfile>,
    #[serde(default)]
    pub contention_disposition: Option<ContentionDispositionProfile>,
    #[serde(default)]
    pub commodity_valuation: Option<CommodityValuationProfile>,
    #[serde(default)]
    pub substitute_preferences: Option<SubstitutePreferences>,
    #[serde(default)]
    pub testimony_trust_profile: Option<TestimonyTrustProfile>,
    #[serde(default)]
    pub route_preference_profile: Option<RoutePreferenceProfile>,
    #[serde(default)]
    pub known_recipes: Option<Vec<String>>,
}

/// Scenario-specific merchandise profile using string names instead of `EntityId`.
///
/// `MerchandiseProfile` in core contains `home_facility: Option<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses a
/// facility/entity name string, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct MerchandiseProfileDef {
    pub sale_kinds: Vec<CommodityKind>,
    #[serde(default)]
    pub home_facility: Option<String>,
}

/// Scenario-specific patrol route using string place names instead of `EntityId`.
///
/// `PatrolRoute` in core contains `assigned_places: Vec<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses
/// place name strings, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct PatrolRouteDef {
    pub assigned_places: Vec<String>,
}

/// Scenario-facing exploration disposition.
///
/// `ExplorationProfile` in core also contains the runtime-only
/// `consecutive_exploration_count`, which must always start at `0` during
/// scenario bootstrap and therefore is not directly authorable in RON.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationProfileDef {
    pub curiosity_weight: Permille,
    pub need_activation_threshold: Permille,
    #[serde(default = "default_exploration_frontier_depth")]
    pub frontier_depth: u16,
    #[serde(default = "default_exploration_acquisition_failure_threshold")]
    pub acquisition_failure_threshold: u8,
    #[serde(default = "default_exploration_arrival_boost")]
    pub exploration_arrival_boost: Permille,
    pub max_consecutive_explorations: u8,
    pub visit_lookback_ticks: u32,
    #[serde(default = "default_negative_survey_damping_window")]
    pub negative_survey_damping_window: u32,
    #[serde(default = "default_negative_survey_damping_strength")]
    pub negative_survey_damping_strength: Permille,
}

const fn default_exploration_frontier_depth() -> u16 {
    2
}

const fn default_exploration_acquisition_failure_threshold() -> u8 {
    3
}

fn default_exploration_arrival_boost() -> Permille {
    Permille::new_unchecked(500)
}

const fn default_negative_survey_damping_window() -> u32 {
    200
}

fn default_negative_survey_damping_strength() -> Permille {
    Permille::new_unchecked(800)
}

impl Default for ExplorationProfileDef {
    fn default() -> Self {
        let profile = ExplorationProfile::default();
        Self::from(profile)
    }
}

impl From<ExplorationProfileDef> for ExplorationProfile {
    fn from(profile: ExplorationProfileDef) -> Self {
        Self {
            curiosity_weight: profile.curiosity_weight,
            need_activation_threshold: profile.need_activation_threshold,
            frontier_depth: profile.frontier_depth,
            acquisition_failure_threshold: profile.acquisition_failure_threshold,
            exploration_arrival_boost: profile.exploration_arrival_boost,
            max_consecutive_explorations: profile.max_consecutive_explorations,
            visit_lookback_ticks: profile.visit_lookback_ticks,
            negative_survey_damping_window: profile.negative_survey_damping_window,
            negative_survey_damping_strength: profile.negative_survey_damping_strength,
            consecutive_exploration_count: 0,
        }
    }
}

impl From<ExplorationProfile> for ExplorationProfileDef {
    fn from(profile: ExplorationProfile) -> Self {
        Self {
            curiosity_weight: profile.curiosity_weight,
            need_activation_threshold: profile.need_activation_threshold,
            frontier_depth: profile.frontier_depth,
            acquisition_failure_threshold: profile.acquisition_failure_threshold,
            exploration_arrival_boost: profile.exploration_arrival_boost,
            max_consecutive_explorations: profile.max_consecutive_explorations,
            visit_lookback_ticks: profile.visit_lookback_ticks,
            negative_survey_damping_window: profile.negative_survey_damping_window,
            negative_survey_damping_strength: profile.negative_survey_damping_strength,
        }
    }
}

/// An item lot to place in the world.
#[derive(Clone, Debug, Deserialize)]
pub struct ItemDef {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub location: String,
    #[serde(default)]
    pub container: bool,
}

/// A workstation facility at a place.
#[derive(Clone, Debug, Deserialize)]
pub struct FacilityDef {
    #[serde(default)]
    pub name: Option<String>,
    pub workstation: WorkstationTag,
    pub location: String,
    #[serde(default)]
    pub merchant_storage: Option<MerchantStorageDef>,
    #[serde(default)]
    pub contention_policy: Option<ContentionPolicy>,
    #[serde(default)]
    pub wash_basin_state: Option<WashBasinStateDef>,
}

/// Optional merchant stock-storage substrate for a facility.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantStorageDef {
    pub owner: String,
    pub stock_capacity: LoadUnits,
    #[serde(default)]
    pub display_capacity: Option<LoadUnits>,
}

/// A resource source at a place.
#[derive(Clone, Debug, Deserialize)]
pub struct ResourceSourceDef {
    pub commodity: CommodityKind,
    pub location: String,
    #[serde(default)]
    pub facility: Option<String>,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub capacity: Quantity,
    #[serde(default = "default_extraction_slots")]
    pub extraction_slots: u8,
    #[serde(default = "default_extraction_duration_ticks")]
    pub extraction_duration_ticks: u32,
}

fn default_extraction_slots() -> u8 {
    1
}

fn default_extraction_duration_ticks() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deserialize with RON extensions that the scenario loader will use.
    fn from_ron_str<'de, T: serde::Deserialize<'de>>(s: &'de str) -> T {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        options.from_str(s).expect("RON deserialization failed")
    }

    #[test]
    fn test_scenario_def_deserialize_minimal() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.seed, 42);
        assert_eq!(def.places.len(), 1);
        assert_eq!(def.places[0].name, "Village");
        assert_eq!(def.places[0].tags, vec![PlaceTag::Village]);
        assert_eq!(def.places[0].visibility_profile, None);
        assert_eq!(def.places[0].sleep_quality, None);
        assert!(def.edges.is_empty());
        assert_eq!(def.agents.len(), 1);
        assert_eq!(def.agents[0].name, "Alice");
        assert_eq!(def.agents[0].location, "Village");
        assert_eq!(def.agents[0].control, ControlSource::Human);
        assert_eq!(def.agents[0].diversification_profile, None);
        assert_eq!(def.agents[0].social_observations, None);
        assert_eq!(def.agents[0].drive_escalation_profile, None);
        assert_eq!(def.agents[0].agenda_profile, None);
        assert!(def.items.is_empty());
        assert!(def.facilities.is_empty());
        assert!(def.artifacts.is_empty());
        assert!(def.resource_sources.is_empty());
        assert_eq!(def.commodity_decay, None);
    }

    #[test]
    fn test_scenario_def_deserializes_place_sleep_quality() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (
                    name: "Hillside Shelter",
                    tags: [Camp],
                    sleep_quality: (
                        shelter: Shelter,
                        ground_comfort: Soft,
                        recovery_modifier: 1250,
                    ),
                ),
            ],
            agents: [
                (name: "Alice", location: "Hillside Shelter", control: Human),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(
            def.places[0].sleep_quality,
            Some(SleepQualityProfileDef {
                shelter: ShelterTag::Shelter,
                ground_comfort: GroundComfortTag::Soft,
                recovery_modifier: 1250,
            })
        );
    }

    #[test]
    fn sleep_quality_profile_def_accepts_above_default_modifier() {
        let def = SleepQualityProfileDef {
            shelter: ShelterTag::Open,
            ground_comfort: GroundComfortTag::Earth,
            recovery_modifier: 1250,
        };

        assert_eq!(
            def.into_profile().unwrap().recovery_modifier,
            SleepRecoveryModifier::new(1250)
        );
    }

    #[test]
    fn test_scenario_def_deserializes_artifact_authors() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
            artifacts: [
                (
                    kind: Notice,
                    issuer: "Alice",
                    location: "Village",
                    expires_at: 18,
                    payload: Notice(ThreatWarning(place: "Village")),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.artifacts.len(), 1);
        let artifact = &def.artifacts[0];
        assert_eq!(artifact.kind, ArtifactKindDef::Notice);
        assert_eq!(artifact.issuer, "Alice");
        assert_eq!(artifact.location, "Village");
        assert_eq!(artifact.expires_at, Some(18));
        assert_eq!(artifact.issuing_authority, None);
        assert_eq!(artifact.jurisdiction, None);
        match &artifact.payload {
            ArtifactPayloadDef::Notice(NoticeTopicDef::ThreatWarning { place }) => {
                assert_eq!(place, "Village");
            }
            ArtifactPayloadDef::Notice(other) => {
                panic!("expected threat-warning notice, got {other:?}")
            }
            ArtifactPayloadDef::Bounty(other) => {
                panic!("expected threat-warning notice, got {other:?}")
            }
        }
    }

    #[test]
    fn test_scenario_def_deserializes_bounty_artifact_payload_and_axes() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
                (name: "Bandit", location: "Village", control: Ai),
            ],
            artifacts: [
                (
                    kind: Bounty,
                    issuer: "Alice",
                    location: "Village",
                    payload: Bounty((
                        target: EliminateEntity(target: "Bandit"),
                        proof_requirement: PhysicalEvidence,
                        reward_commodity: Coin,
                        reward_quantity: 7,
                        reward_source: InstitutionalTreasury(treasury_entity: "Alice"),
                        claim_place: "Village",
                    )),
                    legal_effect: Active(expires_at: Some(99)),
                    actionability: AwaitingProof(required_proof: PhysicalEvidence),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let artifact = &def.artifacts[0];
        assert_eq!(artifact.kind, ArtifactKindDef::Bounty);
        assert_eq!(
            artifact.legal_effect,
            Some(ArtifactLegalEffectDef::Active {
                expires_at: Some(99)
            })
        );
        assert_eq!(
            artifact.actionability,
            Some(ArtifactActionabilityDef::AwaitingProof {
                required_proof: ProofKind::PhysicalEvidence
            })
        );
        match &artifact.payload {
            ArtifactPayloadDef::Bounty(terms) => {
                assert_eq!(
                    terms.target,
                    BountyTargetDef::EliminateEntity {
                        target: "Bandit".into()
                    }
                );
                assert_eq!(terms.reward_quantity, Quantity(7));
            }
            other @ ArtifactPayloadDef::Notice(_) => {
                panic!("expected bounty payload, got {other:?}")
            }
        }
    }

    #[test]
    fn test_scenario_def_deserializes_agent_social_observations() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Alice",
                    location: "Village",
                    control: Human,
                    social_observations: [
                        (
                            place: "Village",
                            observed_tick: 7,
                            source: DirectObservation,
                            detail: WitnessedConflict(actor: "Alice", target: "Alice"),
                        ),
                    ],
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let observations = def.agents[0]
            .social_observations
            .as_ref()
            .expect("social observations should deserialize");
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].place, "Village");
        assert_eq!(observations[0].observed_tick, 7);
        assert_eq!(observations[0].source, PerceptionSource::DirectObservation);
        assert_eq!(
            observations[0].detail,
            SocialObservationDetailDef::WitnessedConflict {
                actor: "Alice".into(),
                target: "Alice".into(),
            }
        );
    }

    #[test]
    fn test_scenario_def_deserializes_survival_health_contract() {
        let ron_str = r#"(
            seed: 42,
            survival_health_contract: (
                max_authored_critical_run_ticks: 123,
                max_idle_window_ticks_with_elevated_need: 17,
                elevated_need_floor: 300,
                required_self_care_families: [Eat, Drink, Sleep, Relieve],
                critical_run_limits: (
                    dirtiness: 999,
                ),
            ),
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let contract = def
            .survival_health_contract
            .expect("survival contract should deserialize");
        assert_eq!(contract.max_authored_critical_run_ticks, 123);
        assert_eq!(contract.max_idle_window_ticks_with_elevated_need, 17);
        assert_eq!(contract.elevated_need_floor, Permille::new(300).unwrap());
        assert_eq!(
            contract.required_self_care_families,
            vec![
                NeedsActionFamily::Eat,
                NeedsActionFamily::Drink,
                NeedsActionFamily::Sleep,
                NeedsActionFamily::Relieve,
            ]
        );
        assert_eq!(
            contract.critical_run_limits,
            Some(SurvivalCriticalRunLimitsDef {
                hunger: None,
                thirst: None,
                fatigue: None,
                bladder: None,
                dirtiness: Some(999),
            })
        );
    }

    #[test]
    fn test_scenario_def_deserializes_diversification_profile() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Alice",
                    location: "Village",
                    control: Human,
                    diversification_profile: (
                        base_curiosity: 400,
                        comfort_threshold: 450,
                        curiosity_buildup_rate: 5,
                        exploration_cooldown_ticks: 60,
                        familiarity_per_visit: 150,
                        familiarity_recovery_per_tick: 2,
                        familiarity_floor: 50,
                        max_exploration_hops: 3,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(
            def.agents[0].diversification_profile,
            Some(DiversificationProfile {
                base_curiosity: Permille::new(400).unwrap(),
                comfort_threshold: Permille::new(450).unwrap(),
                curiosity_buildup_rate: Permille::new(5).unwrap(),
                exploration_cooldown_ticks: 60,
                familiarity_per_visit: Permille::new(150).unwrap(),
                familiarity_recovery_per_tick: Permille::new(2).unwrap(),
                familiarity_floor: Permille::new(50).unwrap(),
                max_exploration_hops: 3,
            })
        );
    }

    #[test]
    fn exploration_profile_def_defaults_negative_survey_damping_fields_when_omitted() {
        let ron_str = r"(
            curiosity_weight: 275,
            need_activation_threshold: 350,
            max_consecutive_explorations: 5,
            visit_lookback_ticks: 17,
        )";

        let profile: ExplorationProfileDef = from_ron_str(ron_str);

        assert_eq!(profile.negative_survey_damping_window, 200);
        assert_eq!(
            profile.negative_survey_damping_strength,
            Permille::new(800).unwrap()
        );
    }

    #[test]
    fn exploration_profile_def_round_trips_negative_survey_damping_fields() {
        let def = ExplorationProfileDef {
            curiosity_weight: Permille::new(275).unwrap(),
            need_activation_threshold: Permille::new(350).unwrap(),
            frontier_depth: 4,
            acquisition_failure_threshold: 6,
            exploration_arrival_boost: Permille::new(650).unwrap(),
            max_consecutive_explorations: 5,
            visit_lookback_ticks: 17,
            negative_survey_damping_window: 123,
            negative_survey_damping_strength: Permille::new(625).unwrap(),
        };

        let profile = ExplorationProfile::from(def);
        assert_eq!(profile.negative_survey_damping_window, 123);
        assert_eq!(
            profile.negative_survey_damping_strength,
            Permille::new(625).unwrap()
        );
        assert_eq!(ExplorationProfileDef::from(profile), def);
    }

    #[test]
    fn test_scenario_def_perception_profile_defaults_observation_budget_when_omitted() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Bob",
                    location: "Village",
                    control: Ai,
                    perception_profile: (
                        entity_activation_threshold: 204,
                        claim_confidence_threshold: 50,
                        observation_buffer_capacity: 6,
                        need_salience_boost: 500,
                        need_salience_urgency_threshold: 500,
                        observation_fidelity: 900,
                        confidence_policy: (
                            direct_observation_base: 980,
                            report_base: 820,
                            rumor_base: 610,
                            inference_base: 430,
                            report_chain_penalty: 45,
                            rumor_chain_penalty: 120,
                            staleness_penalty_per_tick: 4,
                        ),
                        institutional_memory_capacity: 14,
                        consultation_speed_factor: 600,
                        contradiction_tolerance: 250,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let perception = def.agents[0]
            .perception_profile
            .expect("perception profile should deserialize");

        assert_eq!(perception.observation_budget, 24);
    }

    #[test]
    fn test_scenario_def_deserialize_full() {
        let ron_str = r#"(
            seed: 123,
            places: [
                (name: "Town", tags: [Village, Store]),
                (name: "Forest", tags: [Forest]),
            ],
            edges: [
                (from: "Town", to: "Forest", travel_ticks: 3, bidirectional: true),
            ],
            agents: [
                (
                    name: "Bob",
                    location: "Town",
                    control: Ai,
                    needs: (
                        hunger: 100,
                        thirst: 200,
                        fatigue: 50,
                        bladder: 0,
                        dirtiness: 0,
                    ),
                    combat_profile: (
                        wound_capacity: 800,
                        incapacitation_threshold: 700,
                        attack_skill: 500,
                        guard_skill: 400,
                        defend_bonus: 100,
                        natural_clot_resistance: 300,
                        natural_recovery_rate: 50,
                        unarmed_wound_severity: 200,
                        unarmed_bleed_rate: 100,
                        unarmed_attack_ticks: 3,
                        defend_stance_ticks: 10,
                    ),
                    utility_profile: (
                        hunger_weight: 500,
                        thirst_weight: 500,
                        fatigue_weight: 500,
                        bladder_weight: 500,
                        dirtiness_weight: 500,
                        pain_weight: 500,
                        danger_weight: 500,
                        enterprise_weight: 500,
                        social_weight: 200,
                        activity_awareness_weight: 200,
                        side_benefit_weight: 100,
                        bounty_posting_weight: 0,
                        notice_posting_weight: 0,
                        courage: 500,
                        care_weight: 200,
                    ),
                    merchandise_profile: (
                        sale_kinds: [Apple, Bread],
                        home_facility: "Town",
                    ),
                    trade_disposition: (
                        negotiation_round_ticks: 2,
                        initial_offer_bias: 600,
                        concession_rate: 100,
                        rejection_escalation_rate: 200,
                        demand_memory_retention_ticks: 50,
                        market_presence_ticks: 30,
                    ),
                    perception_profile: (
                        entity_activation_threshold: 204,
                        claim_confidence_threshold: 50,
                        observation_buffer_capacity: 6,
                        observation_budget: 7,
                        need_salience_boost: 500,
                        need_salience_urgency_threshold: 500,
                        observation_fidelity: 900,
                        confidence_policy: (
                            direct_observation_base: 980,
                            report_base: 820,
                            rumor_base: 610,
                            inference_base: 430,
                            report_chain_penalty: 45,
                            rumor_chain_penalty: 120,
                            staleness_penalty_per_tick: 4,
                        ),
                        institutional_memory_capacity: 14,
                        consultation_speed_factor: 600,
                        contradiction_tolerance: 250,
                    ),
                    drive_thresholds: (
                        hunger: (low: 150, medium: 300, high: 600, critical: 850),
                        thirst: (low: 160, medium: 320, high: 610, critical: 860),
                        fatigue: (low: 170, medium: 340, high: 620, critical: 870),
                        bladder: (low: 180, medium: 360, high: 630, critical: 880),
                        dirtiness: (low: 190, medium: 380, high: 640, critical: 890),
                        pain: (low: 120, medium: 240, high: 520, critical: 800),
                        danger: (low: 80, medium: 220, high: 480, critical: 760),
                    ),
                    drive_escalation_profile: (
                        per_need: {
                            Dirtiness: (
                                start_after_ticks: 40,
                                growth_per_tick: 25,
                                max_multiplier: 2200,
                            ),
                        },
                        default_per_need: (
                            start_after_ticks: 80,
                            growth_per_tick: 15,
                            max_multiplier: 1800,
                        ),
                    ),
                    exploration_profile: (
                        curiosity_weight: 275,
                        need_activation_threshold: 350,
                        max_consecutive_explorations: 5,
                        visit_lookback_ticks: 17,
                    ),
                    obligation_satiation_profile: (
                        satiation_threshold: 4,
                        window_ticks: 96,
                        decay_per_execution: 150,
                        satiation_floor: 125,
                    ),
                    theft_disposition: (
                        steal_duration_ticks: 6,
                        theft_motive_weight: 620,
                        witness_risk_penalty: 180,
                    ),
                    patrol_route: (
                        assigned_places: ["Town", "Forest"],
                    ),
                ),
            ],
            items: [
                (commodity: Apple, quantity: 10, location: "Town", container: false),
                (commodity: Sword, quantity: 1, location: "Bob"),
            ],
            facilities: [
                (
                    name: "Town Stall",
                    workstation: Forge,
                    location: "Town",
                    merchant_storage: (
                        owner: "Bob",
                        stock_capacity: 200,
                        display_capacity: 100,
                    ),
                ),
            ],
            resource_sources: [
                (commodity: Apple, location: "Forest", regeneration_ticks_per_unit: Some(5), capacity: 20),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.seed, 123);
        assert_eq!(def.places.len(), 2);
        assert_eq!(def.edges.len(), 1);
        assert_eq!(def.edges[0].from, "Town");
        assert_eq!(def.edges[0].to, "Forest");
        assert_eq!(def.edges[0].travel_ticks, 3);
        assert!(def.edges[0].bidirectional);
        assert_eq!(def.agents.len(), 1);

        let bob = &def.agents[0];
        assert_eq!(bob.name, "Bob");
        assert_eq!(bob.control, ControlSource::Ai);
        assert!(bob.needs.is_some());
        assert!(bob.combat_profile.is_some());
        assert!(bob.utility_profile.is_some());
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .activity_awareness_weight
                .value(),
            200
        );
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .bounty_posting_weight
                .value(),
            0
        );
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .side_benefit_weight
                .value(),
            100
        );
        assert!(bob.merchandise_profile.is_some());
        let merch = bob.merchandise_profile.as_ref().unwrap();
        assert_eq!(
            merch.sale_kinds,
            vec![CommodityKind::Apple, CommodityKind::Bread]
        );
        assert_eq!(merch.home_facility, Some("Town".to_string()));
        assert!(bob.trade_disposition.is_some());
        assert!(bob.perception_profile.is_some());
        let perception = bob.perception_profile.unwrap();
        assert_eq!(perception.entity_activation_threshold.value(), 204);
        assert_eq!(perception.claim_confidence_threshold.value(), 50);
        assert_eq!(perception.observation_buffer_capacity, 6);
        assert_eq!(perception.observation_budget, 7);
        assert!(bob.drive_thresholds.is_some());
        assert_eq!(bob.drive_thresholds.unwrap().hunger.low().value(), 150);
        assert_eq!(def.facilities.len(), 1);
        let facility = &def.facilities[0];
        assert_eq!(facility.name.as_deref(), Some("Town Stall"));
        let merchant_storage = facility
            .merchant_storage
            .as_ref()
            .expect("merchant storage should deserialize");
        assert_eq!(merchant_storage.owner, "Bob");
        assert_eq!(merchant_storage.stock_capacity, LoadUnits(200));
        assert_eq!(merchant_storage.display_capacity, Some(LoadUnits(100)));
        assert_eq!(
            bob.drive_escalation_profile,
            Some(worldwake_core::DriveEscalationProfile {
                per_need: std::collections::BTreeMap::from([(
                    worldwake_core::HomeostaticNeedId::Dirtiness,
                    worldwake_core::DriveEscalationParams {
                        start_after_ticks: 40,
                        growth_per_tick: Permille::new(25).unwrap(),
                        max_multiplier: worldwake_core::MultiplierPermille::new(2200).unwrap(),
                    },
                )]),
                default_per_need: worldwake_core::DriveEscalationParams {
                    start_after_ticks: 80,
                    growth_per_tick: Permille::new(15).unwrap(),
                    max_multiplier: worldwake_core::MultiplierPermille::new(1800).unwrap(),
                },
            })
        );
        assert_eq!(
            bob.exploration_profile,
            Some(ExplorationProfileDef {
                curiosity_weight: Permille::new(275).unwrap(),
                need_activation_threshold: Permille::new(350).unwrap(),
                frontier_depth: 2,
                acquisition_failure_threshold: 3,
                exploration_arrival_boost: Permille::new(500).unwrap(),
                max_consecutive_explorations: 5,
                visit_lookback_ticks: 17,
                negative_survey_damping_window: 200,
                negative_survey_damping_strength: Permille::new(800).unwrap(),
            })
        );
        assert_eq!(
            bob.obligation_satiation_profile,
            Some(ObligationSatiationProfile {
                satiation_threshold: 4,
                window_ticks: 96,
                decay_per_execution: Permille::new(150).unwrap(),
                satiation_floor: Permille::new(125).unwrap(),
            })
        );
        assert!(bob.theft_disposition.is_some());
        assert_eq!(
            bob.theft_disposition
                .as_ref()
                .unwrap()
                .theft_motive_weight
                .value(),
            620
        );
        assert!(bob.patrol_route.is_some());
        assert_eq!(
            bob.patrol_route.as_ref().unwrap().assigned_places,
            vec!["Town".to_string(), "Forest".to_string()]
        );

        assert_eq!(def.items.len(), 2);
        assert!(!def.items[0].container);
        assert_eq!(def.facilities.len(), 1);
        assert_eq!(def.facilities[0].workstation, WorkstationTag::Forge);
        assert_eq!(def.resource_sources.len(), 1);
        assert_eq!(def.resource_sources[0].capacity, Quantity(20));
    }

    #[test]
    fn test_scenario_def_commodity_decay_omitted_field_stays_none() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);

        assert_eq!(def.commodity_decay, None);
    }

    #[test]
    fn test_scenario_def_commodity_decay_deserializes_when_present() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
            commodity_decay: {
                Waste: 200,
                Apple: 720,
            },
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);

        assert_eq!(
            def.commodity_decay,
            Some(CommodityDecayMap::from([
                (CommodityKind::Apple, NonZeroU32::new(720).unwrap()),
                (CommodityKind::Waste, NonZeroU32::new(200).unwrap()),
            ]))
        );
    }

    #[test]
    fn test_place_def_deserializes_visibility_profile() {
        let ron_str = r#"(
            seed: 7,
            places: [
                (
                    name: "Forest",
                    tags: [Forest],
                    visibility_profile: (
                        base_concealment: 400,
                    ),
                ),
            ],
            agents: [
                (name: "Scout", location: "Forest", control: Ai),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(
            def.places[0].visibility_profile,
            Some(PlaceVisibilityProfile {
                base_concealment: Permille::new(400).unwrap(),
            })
        );
    }

    #[test]
    fn test_exploration_profile_def_rejects_runtime_counter_field() {
        let ron_str = r#"(
            seed: 7,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Scout",
                    location: "Village",
                    control: Ai,
                    exploration_profile: (
                        curiosity_weight: 275,
                        need_activation_threshold: 350,
                        frontier_depth: 4,
                        acquisition_failure_threshold: 6,
                        exploration_arrival_boost: 650,
                        max_consecutive_explorations: 5,
                        visit_lookback_ticks: 17,
                        consecutive_exploration_count: 1,
                    ),
                ),
            ],
        )"#;

        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let error = options
            .from_str::<ScenarioDef>(ron_str)
            .expect_err("runtime-only exploration counter should not deserialize");

        assert!(
            error
                .to_string()
                .contains("Unexpected field named `consecutive_exploration_count`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_exploration_profile_def_defaults_new_fields_when_omitted() {
        let ron_str = r#"(
            seed: 7,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Scout",
                    location: "Village",
                    control: Ai,
                    exploration_profile: (
                        curiosity_weight: 275,
                        need_activation_threshold: 350,
                        max_consecutive_explorations: 5,
                        visit_lookback_ticks: 17,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let exploration = def.agents[0].exploration_profile.unwrap();

        assert_eq!(exploration.frontier_depth, 2);
        assert_eq!(exploration.acquisition_failure_threshold, 3);
        assert_eq!(
            exploration.exploration_arrival_boost,
            Permille::new(500).unwrap()
        );
        assert_eq!(exploration.negative_survey_damping_window, 200);
        assert_eq!(
            exploration.negative_survey_damping_strength,
            Permille::new(800).unwrap()
        );
    }

    #[test]
    fn test_place_def_visibility_profile_defaults_to_none() {
        let ron_str = r#"(
            seed: 8,
            places: [
                (name: "Square", tags: [Village]),
            ],
            agents: [
                (name: "Watcher", location: "Square", control: Ai),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.places[0].visibility_profile, None);
    }

    #[test]
    fn test_agent_def_default_optional_fields() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (name: "Minimal", location: "Nowhere", control: None),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let agent = &def.agents[0];
        assert_eq!(agent.name, "Minimal");
        assert_eq!(agent.location, "Nowhere");
        assert_eq!(agent.control, ControlSource::None);
        assert!(agent.needs.is_none());
        assert!(agent.combat_profile.is_none());
        assert!(agent.utility_profile.is_none());
        assert!(agent.artifact_posting_profile.is_none());
        assert!(agent.merchandise_profile.is_none());
        assert!(agent.trade_disposition.is_none());
        assert!(agent.perception_profile.is_none());
        assert!(agent.tell_profile.is_none());
        assert!(agent.cognitive_profile.is_none());
        assert!(agent.risk_weight_profile.is_none());
        assert!(agent.law_abiding_profile.is_none());
        assert!(agent.agenda_profile.is_none());
        assert!(agent.execution_budget.is_none());
        assert!(agent.epistemic_disposition.is_none());
        assert!(agent.intention_disposition.is_none());
        assert!(agent.communication_profile.is_none());
        assert!(agent.preference_profile.is_none());
        assert!(agent.expectation_store.is_none());
        assert!(agent.last_seen_memory.is_none());
        assert!(agent.obligation_satiation_profile.is_none());
        assert!(agent.drive_thresholds.is_none());
        assert!(agent.drive_escalation_profile.is_none());
        assert!(agent.metabolism_profile.is_none());
        assert!(agent.carry_capacity.is_none());
        assert!(agent.theft_disposition.is_none());
        assert!(agent.justice_disposition.is_none());
        assert!(agent.violation_disposition.is_none());
        assert!(agent.patrol_profile.is_none());
        assert!(agent.patrol_route.is_none());
        assert!(agent.pursuit_profile.is_none());
        assert!(agent.contention_disposition.is_none());
        assert!(agent.commodity_valuation.is_none());
        assert!(agent.substitute_preferences.is_none());
        assert!(agent.testimony_trust_profile.is_none());
        assert!(agent.route_preference_profile.is_none());
    }

    #[test]
    fn test_scenario_def_cognitive_profile_missing_new_field_uses_default() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Planner",
                    location: "Nowhere",
                    control: Ai,
                    cognitive_profile: (
                        max_plan_depth: 10,
                        snapshot_travel_horizon: 6,
                        max_node_expansions: 300,
                        switch_margin: 100,
                        planning_switch_margin: 150,
                        transient_block_ticks: 20,
                        structural_block_ticks: 200,
                        initial_cooldown_ticks: 4,
                        max_cooldown_ticks: 64,
                        landmark_extraction_depth: 3,
                    ),
                    agenda_profile: (
                        pending_capacity: 20,
                        suspended_capacity: 4,
                        revive_cooldown_ticks: 2,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let cognitive = def.agents[0]
            .cognitive_profile
            .expect("cognitive profile should deserialize");

        assert_eq!(
            cognitive.max_candidates_per_expansion,
            CognitiveProfile::default().max_candidates_per_expansion
        );
        assert_eq!(cognitive.max_plan_depth, 10);
        assert_eq!(cognitive.max_travel_candidates_per_expansion, None);
        assert_eq!(cognitive.landmark_extraction_depth, 3);
        assert_eq!(cognitive.survey_memory_capacity, 24);
        assert_eq!(cognitive.survey_memory_retention_ticks, 300);
        assert_eq!(
            def.agents[0].agenda_profile,
            Some(AgendaProfile {
                pending_capacity: 20,
                suspended_capacity: 4,
                revive_cooldown_ticks: 2,
            })
        );
    }

    #[test]
    fn test_scenario_def_opportunity_profiles_deserialize() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Planner",
                    location: "Nowhere",
                    control: Ai,
                    risk_weight_profile: (
                        theft_aversion: 250,
                        exposure_aversion: 500,
                        threat_aversion: 750,
                    ),
                    law_abiding_profile: (
                        criminal_threshold: 600,
                        social_norm_weight: 350,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let agent = &def.agents[0];

        assert_eq!(
            agent.risk_weight_profile,
            Some(RiskWeightProfile {
                theft_aversion: Permille::new_unchecked(250),
                exposure_aversion: Permille::new_unchecked(500),
                threat_aversion: Permille::new_unchecked(750),
            })
        );
        assert_eq!(
            agent.law_abiding_profile,
            Some(LawAbidingProfile {
                criminal_threshold: Permille::new_unchecked(600),
                social_norm_weight: Permille::new_unchecked(350),
            })
        );
    }

    #[test]
    fn test_scenario_def_artifact_posting_profile_deserializes_when_present() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Herald",
                    location: "Nowhere",
                    control: Ai,
                    artifact_posting_profile: (
                        threat_warning_ttl: 12,
                        office_vacancy_ttl: 34,
                        bounty_ttl: 56,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let profile = def.agents[0]
            .artifact_posting_profile
            .clone()
            .expect("artifact posting profile should deserialize");

        assert_eq!(
            profile,
            ArtifactPostingProfile {
                threat_warning_ttl: 12,
                office_vacancy_ttl: 34,
                bounty_ttl: 56,
            }
        );
    }

    #[test]
    fn test_scenario_def_artifact_posting_profile_omitted_field_stays_none() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Herald",
                    location: "Nowhere",
                    control: Ai,
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.agents[0].artifact_posting_profile, None);
    }

    #[test]
    fn test_patrol_route_def_deserializes_place_names() {
        let route: PatrolRouteDef = from_ron_str(
            r#"(
                assigned_places: ["Gate", "Market"],
            )"#,
        );

        assert_eq!(
            route.assigned_places,
            vec!["Gate".to_string(), "Market".to_string()]
        );
    }

    #[test]
    fn test_edge_def_bidirectional_default() {
        let ron_str = r#"(
            seed: 1,
            places: [
                (name: "A", tags: []),
                (name: "B", tags: []),
            ],
            edges: [
                (from: "A", to: "B", travel_ticks: 2),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert!(
            def.edges[0].bidirectional,
            "bidirectional should default to true"
        );
    }

    #[test]
    fn scenario_def_resource_source_defaults_to_one_slot() {
        let ron_str = r#"(
            seed: 1,
            places: [
                (name: "Orchard", tags: [Forest]),
            ],
            resource_sources: [
                (
                    commodity: Apple,
                    location: "Orchard",
                    regeneration_ticks_per_unit: Some(5),
                    capacity: 12,
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.resource_sources.len(), 1);
        assert_eq!(
            def.resource_sources[0].extraction_slots, 1,
            "extraction_slots default must be 1"
        );
        assert_eq!(
            def.resource_sources[0].extraction_duration_ticks, 1,
            "extraction_duration_ticks default must be 1"
        );
    }

    #[test]
    fn scenario_def_resource_source_explicit_slots() {
        let ron_str = r#"(
            seed: 1,
            places: [
                (name: "RiverBank", tags: [Forest]),
            ],
            resource_sources: [
                (
                    commodity: Water,
                    location: "RiverBank",
                    regeneration_ticks_per_unit: Some(3),
                    capacity: 30,
                    extraction_slots: 5,
                    extraction_duration_ticks: 4,
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.resource_sources[0].extraction_slots, 5);
        assert_eq!(def.resource_sources[0].extraction_duration_ticks, 4);
    }
}
