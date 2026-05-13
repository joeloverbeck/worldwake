use crate::goal_model::GoalPriorityClass;
use crate::goal_policy::{
    FreeInterruptRole, GoalFamilyPolicy, PenaltyInterruptEligibility, SuppressionRule,
};
use crate::interrupts::InterruptTrigger;
use crate::{PlannerOpKind, RankedGoalProvenanceFamily, goal_dispatch_key::GoalDispatchKey};
use serde::{Deserialize, Serialize};
use worldwake_core::HomeostaticNeedId;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InvalidationStrategy {
    NoOpinion,
    CommodityOnly,
    AcquireCommodity,
    AcquireRestock,
    NeedWithFacilities(HomeostaticNeedId),
    NeedWithPosition(HomeostaticNeedId),
    CombatTarget,
    DangerReduction,
    FactionRegroup,
    TreatWounds,
    ProduceCommodity,
    PositionAndCommodity,
    PositionCommodityAndCoin,
    PositionAndTargetDead,
    BountyActive,
    StealTargetState,
    ClaimOffice,
    SupportCandidateForOffice,
    InvestigateViolation,
    Patrol,
    PunishAccused,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeasibilityStrategy {
    OwnedCommodityCheck,
    EvidencePlaceLocal,
    AlwaysLikely,
    CommodityPresenceCheck,
    ColocationOrDead,
    NoOpinion,
    SellCheck,
    CargoDestinationCheck,
    CorpseBurialCheck,
    PlaceMatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FrontierExhaustionStrategy {
    PermanentUntilInvalidator,
    CooldownRetry,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Authority {
    Gate,
    HintOnly,
}

pub struct GoalDispatchDeclaration {
    pub trace_label: &'static str,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    pub relevant_ops: &'static [PlannerOpKind],
    pub invalidation_strategy: InvalidationStrategy,
    pub feasibility_strategy: FeasibilityStrategy,
    pub frontier_exhaustion_strategy: FrontierExhaustionStrategy,
    pub family_policy: GoalFamilyPolicy,
    /// `PlannerOpKind` values that constitute a direct progress barrier for this goal.
    /// Does not include `QueueForFacilityUse` (goal-family membership check),
    /// `ConsumeOwnedCommodity`/`MoveCargo` (special case), or `is_materialization_barrier`
    /// (step-property check) — those remain in `is_progress_barrier()`.
    pub progress_barrier_ops: &'static [PlannerOpKind],
}

impl GoalDispatchDeclaration {
    pub fn relevant_ops_authority(&self) -> Authority {
        Authority::HintOnly
    }
}

const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
const ACQUIRE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SLEEP_OPS: &[PlannerOpKind] = &[PlannerOpKind::Sleep];
const RELIEVE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Relieve, PlannerOpKind::Travel];
const WASH_OPS: &[PlannerOpKind] = &[PlannerOpKind::Wash, PlannerOpKind::Travel];
const FREE_CARRY_CAPACITY_OPS: &[PlannerOpKind] = &[PlannerOpKind::DropItem];
const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Attack];
const RAID_TARGET_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Attack];
const REDUCE_DANGER_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Defend,
    PlannerOpKind::Heal,
];
const REGROUP_WITH_FACTION_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel];
const ESTABLISH_BANDIT_CAMP_OPS: &[PlannerOpKind] = &[PlannerOpKind::EstablishCamp];
const TREAT_WOUNDS_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Heal,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::Harvest,
];
const SEARCH_FOR_MISSING_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::AskAboutPerson,
    PlannerOpKind::SearchPlace,
];
const REPORT_MISSING_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::ReportMissing];
const REPORT_FOUND_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::ReportFound];
const ESCORT_TO_SAFETY_OPS: &[PlannerOpKind] =
    &[PlannerOpKind::Travel, PlannerOpKind::EscortToSafety];
const PRODUCE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::StockManagement,
    PlannerOpKind::StaffMarket,
];
const RESTOCK_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const MOVE_CARGO_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::StockManagement,
];
const LOOT_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Loot,
];
const BURY_OPS: &[PlannerOpKind] = &[PlannerOpKind::QueueForFacilityUse, PlannerOpKind::Bury];
const FULFILL_BOUNTY_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Attack,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::StockManagement,
    PlannerOpKind::ClaimBounty,
];
const POST_BOUNTY_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::PostBounty];
const POST_NOTICE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::PostNotice];
const SHARE_BELIEF_OPS: &[PlannerOpKind] = &[PlannerOpKind::Tell];
const ASK_WITNESS_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::AskWitness];
const CLAIM_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::DeclareSupport,
    PlannerOpKind::PressForceClaim,
];
const SUPPORT_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::ConsultRecord,
    PlannerOpKind::DeclareSupport,
];
const INVESTIGATE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Investigate];
const PATROL_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Patrol];
const EXPLORE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel];
const ACCUSE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Accuse];
const FINE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Fine];
const EXILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Exile];

// ---------------------------------------------------------------------------
// Family policy constants
// ---------------------------------------------------------------------------

const SELF_CARE_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
        trigger: InterruptTrigger::CriticalSurvival,
    },
    free_interrupt: FreeInterruptRole::Reactive,
};

const DANGER_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
        trigger: InterruptTrigger::CriticalDanger,
    },
    free_interrupt: FreeInterruptRole::Reactive,
};

const CARE_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Reactive,
};

const ENTERPRISE_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};

const OPPORTUNISTIC_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Opportunistic,
};

const SOCIAL_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};

const THEFT_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::Never,
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};

// ShareBelief sub-variant policies
const SHARE_BELIEF_ALARM_POLICY: GoalFamilyPolicy = ENTERPRISE_POLICY;

const SHARE_BELIEF_TESTIMONY_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};

const EPISTEMIC_SENSING_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
    penalty_interrupt: PenaltyInterruptEligibility::Never,
    free_interrupt: FreeInterruptRole::Normal,
};

const SHARE_BELIEF_GOSSIP_POLICY: GoalFamilyPolicy = SOCIAL_POLICY;

// ---------------------------------------------------------------------------
// Progress barrier op constants (direct per-goal-kind barriers only)
// ---------------------------------------------------------------------------

const TELL_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Tell];
const INVESTIGATE_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Investigate];
const SEARCH_PLACE_BARRIER: &[PlannerOpKind] =
    &[PlannerOpKind::SearchPlace, PlannerOpKind::AskAboutPerson];
const REPORT_MISSING_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::ReportMissing];
const REPORT_FOUND_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::ReportFound];
const PATROL_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Patrol];
const ESCORT_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::EscortToSafety];
const CLAIM_BOUNTY_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::ClaimBounty];
const ACCUSE_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Accuse];
const STAFF_MARKET_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::StaffMarket];
const FINE_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Fine];
const EXILE_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::Exile];
const ASK_WITNESS_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::AskWitness];
const POST_BOUNTY_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::PostBounty];
const POST_NOTICE_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::PostNotice];
const OFFICE_CLAIM_BARRIER: &[PlannerOpKind] = &[
    PlannerOpKind::DeclareSupport,
    PlannerOpKind::PressForceClaim,
];
const NO_BARRIER: &[PlannerOpKind] = &[];

// ---------------------------------------------------------------------------
// Declaration constants
// ---------------------------------------------------------------------------

static DECL_CONSUME_OWNED_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ConsumeOwnedCommodity",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: CONSUME_OPS,
    invalidation_strategy: InvalidationStrategy::CommodityOnly,
    feasibility_strategy: FeasibilityStrategy::OwnedCommodityCheck,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_ACQUIRE_SELF_CONSUME: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(SelfConsume)",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireCommodity,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::CooldownRetry,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_ACQUIRE_RECIPE_INPUT: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(RecipeInput)",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireCommodity,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_ACQUIRE_RESTOCK: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(Restock)",
    provenance_family: None,
    relevant_ops: ACQUIRE_OPS,
    invalidation_strategy: InvalidationStrategy::AcquireRestock,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_SLEEP: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Sleep",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: SLEEP_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithFacilities(HomeostaticNeedId::Fatigue),
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::CooldownRetry,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: SLEEP_OPS,
};
static DECL_RELIEVE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Relieve",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: RELIEVE_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithPosition(HomeostaticNeedId::Bladder),
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_WASH: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Wash",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: WASH_OPS,
    invalidation_strategy: InvalidationStrategy::NeedWithFacilities(HomeostaticNeedId::Dirtiness),
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_FREE_CARRY_CAPACITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "FreeCarryCapacity",
    provenance_family: None,
    relevant_ops: FREE_CARRY_CAPACITY_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: FREE_CARRY_CAPACITY_OPS,
};
static DECL_ENGAGE_HOSTILE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "EngageHostile",
    provenance_family: Some(RankedGoalProvenanceFamily::Danger),
    relevant_ops: ENGAGE_HOSTILE_OPS,
    invalidation_strategy: InvalidationStrategy::CombatTarget,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_RAID_TARGET: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "RaidTarget",
    provenance_family: None,
    relevant_ops: RAID_TARGET_OPS,
    invalidation_strategy: InvalidationStrategy::CombatTarget,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_REDUCE_DANGER: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ReduceDanger",
    provenance_family: Some(RankedGoalProvenanceFamily::Danger),
    relevant_ops: REDUCE_DANGER_OPS,
    invalidation_strategy: InvalidationStrategy::DangerReduction,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: DANGER_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_REGROUP_WITH_FACTION: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "RegroupWithFaction",
    provenance_family: None,
    relevant_ops: REGROUP_WITH_FACTION_OPS,
    invalidation_strategy: InvalidationStrategy::FactionRegroup,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_ESTABLISH_BANDIT_CAMP: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "EstablishBanditCamp",
    provenance_family: None,
    relevant_ops: ESTABLISH_BANDIT_CAMP_OPS,
    invalidation_strategy: InvalidationStrategy::FactionRegroup,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_TREAT_WOUNDS: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "TreatWounds",
    provenance_family: None,
    relevant_ops: TREAT_WOUNDS_OPS,
    invalidation_strategy: InvalidationStrategy::TreatWounds,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_SEARCH_FOR_MISSING: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "SearchForMissing",
    provenance_family: None,
    relevant_ops: SEARCH_FOR_MISSING_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: SEARCH_PLACE_BARRIER,
};
static DECL_REPORT_MISSING: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ReportMissing",
    provenance_family: None,
    relevant_ops: REPORT_MISSING_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: REPORT_MISSING_BARRIER,
};
static DECL_REPORT_FOUND: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ReportFound",
    provenance_family: None,
    relevant_ops: REPORT_FOUND_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: REPORT_FOUND_BARRIER,
};
static DECL_ESCORT_TO_SAFETY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "EscortToSafety",
    provenance_family: None,
    relevant_ops: ESCORT_TO_SAFETY_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: ESCORT_BARRIER,
};
static DECL_PRODUCE_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ProduceCommodity",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: PRODUCE_OPS,
    invalidation_strategy: InvalidationStrategy::ProduceCommodity,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_SELL_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "SellCommodity",
    provenance_family: None,
    relevant_ops: SELL_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndCommodity,
    feasibility_strategy: FeasibilityStrategy::SellCheck,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: STAFF_MARKET_BARRIER,
};
static DECL_RESTOCK_COMMODITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "RestockCommodity",
    provenance_family: None,
    relevant_ops: RESTOCK_OPS,
    invalidation_strategy: InvalidationStrategy::PositionCommodityAndCoin,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_MOVE_CARGO: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "MoveCargo",
    provenance_family: None,
    relevant_ops: MOVE_CARGO_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndCommodity,
    feasibility_strategy: FeasibilityStrategy::CargoDestinationCheck,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_LOOT_CORPSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "LootCorpse",
    provenance_family: None,
    relevant_ops: LOOT_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: OPPORTUNISTIC_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_BURY_CORPSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "BuryCorpse",
    provenance_family: None,
    relevant_ops: BURY_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::CorpseBurialCheck,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_FULFILL_BOUNTY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "FulfillBounty",
    provenance_family: None,
    relevant_ops: FULFILL_BOUNTY_OPS,
    invalidation_strategy: InvalidationStrategy::BountyActive,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: CLAIM_BOUNTY_BARRIER,
};
static DECL_POST_BOUNTY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PostBounty",
    provenance_family: None,
    relevant_ops: POST_BOUNTY_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: POST_BOUNTY_BARRIER,
};
static DECL_POST_NOTICE_WARNING: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PostNotice(Warning)",
    provenance_family: None,
    relevant_ops: POST_NOTICE_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: ENTERPRISE_POLICY,
    progress_barrier_ops: POST_NOTICE_BARRIER,
};
static DECL_POST_NOTICE_OTHER: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PostNotice(Other)",
    provenance_family: None,
    relevant_ops: POST_NOTICE_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: POST_NOTICE_BARRIER,
};
static DECL_SHARE_BELIEF_ALARM: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ShareBelief(Alarm)",
    provenance_family: None,
    relevant_ops: SHARE_BELIEF_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SHARE_BELIEF_ALARM_POLICY,
    progress_barrier_ops: TELL_BARRIER,
};
static DECL_SHARE_BELIEF_TESTIMONY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ShareBelief(Testimony)",
    provenance_family: None,
    relevant_ops: SHARE_BELIEF_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SHARE_BELIEF_TESTIMONY_POLICY,
    progress_barrier_ops: TELL_BARRIER,
};
static DECL_SHARE_BELIEF_GOSSIP: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ShareBelief(Gossip)",
    provenance_family: None,
    relevant_ops: SHARE_BELIEF_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SHARE_BELIEF_GOSSIP_POLICY,
    progress_barrier_ops: TELL_BARRIER,
};
static DECL_ASK_WITNESS: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AskWitness",
    provenance_family: Some(RankedGoalProvenanceFamily::EpistemicSensing),
    relevant_ops: ASK_WITNESS_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: EPISTEMIC_SENSING_POLICY,
    progress_barrier_ops: ASK_WITNESS_BARRIER,
};
static DECL_CLAIM_OFFICE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ClaimOffice",
    provenance_family: None,
    relevant_ops: CLAIM_OFFICE_OPS,
    invalidation_strategy: InvalidationStrategy::ClaimOffice,
    feasibility_strategy: FeasibilityStrategy::EvidencePlaceLocal,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: OFFICE_CLAIM_BARRIER,
};
static DECL_SUPPORT_CANDIDATE_FOR_OFFICE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "SupportCandidateForOffice",
    provenance_family: None,
    relevant_ops: SUPPORT_OFFICE_OPS,
    invalidation_strategy: InvalidationStrategy::SupportCandidateForOffice,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: OFFICE_CLAIM_BARRIER,
};
static DECL_INVESTIGATE_VIOLATION: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "InvestigateViolation",
    provenance_family: None,
    relevant_ops: INVESTIGATE_OPS,
    invalidation_strategy: InvalidationStrategy::InvestigateViolation,
    feasibility_strategy: FeasibilityStrategy::PlaceMatch,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: INVESTIGATE_BARRIER,
};
static DECL_PATROL: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Patrol",
    provenance_family: None,
    relevant_ops: PATROL_OPS,
    invalidation_strategy: InvalidationStrategy::Patrol,
    feasibility_strategy: FeasibilityStrategy::PlaceMatch,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::CooldownRetry,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: PATROL_BARRIER,
};
static DECL_EXPLORE_LOCATION: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "ExploreLocation",
    provenance_family: None,
    relevant_ops: EXPLORE_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::PlaceMatch,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_STEAL_ITEM: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "StealItem",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: MOVE_CARGO_OPS,
    invalidation_strategy: InvalidationStrategy::StealTargetState,
    feasibility_strategy: FeasibilityStrategy::NoOpinion,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: THEFT_POLICY,
    progress_barrier_ops: NO_BARRIER,
};
static DECL_ACCUSE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "Accuse",
    provenance_family: None,
    relevant_ops: ACCUSE_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: ACCUSE_BARRIER,
};
static DECL_PUNISH_FINE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PunishAccused(Fine)",
    provenance_family: None,
    relevant_ops: FINE_OPS,
    invalidation_strategy: InvalidationStrategy::PunishAccused,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: FINE_BARRIER,
};
static DECL_PUNISH_EXILE: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "PunishAccused(Exile)",
    provenance_family: None,
    relevant_ops: EXILE_OPS,
    invalidation_strategy: InvalidationStrategy::PunishAccused,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: SOCIAL_POLICY,
    progress_barrier_ops: EXILE_BARRIER,
};

impl GoalDispatchKey {
    #[must_use]
    pub const fn declaration(&self) -> &'static GoalDispatchDeclaration {
        match self {
            Self::ConsumeOwnedCommodity => &DECL_CONSUME_OWNED_COMMODITY,
            Self::AcquireSelfConsume => &DECL_ACQUIRE_SELF_CONSUME,
            Self::AcquireRecipeInput => &DECL_ACQUIRE_RECIPE_INPUT,
            Self::AcquireRestock => &DECL_ACQUIRE_RESTOCK,
            Self::Sleep => &DECL_SLEEP,
            Self::Relieve => &DECL_RELIEVE,
            Self::Wash => &DECL_WASH,
            Self::FreeCarryCapacity => &DECL_FREE_CARRY_CAPACITY,
            Self::EngageHostile => &DECL_ENGAGE_HOSTILE,
            Self::RaidTarget => &DECL_RAID_TARGET,
            Self::ReduceDanger => &DECL_REDUCE_DANGER,
            Self::RegroupWithFaction => &DECL_REGROUP_WITH_FACTION,
            Self::EstablishBanditCamp => &DECL_ESTABLISH_BANDIT_CAMP,
            Self::TreatWounds => &DECL_TREAT_WOUNDS,
            Self::SearchForMissing => &DECL_SEARCH_FOR_MISSING,
            Self::ReportMissing => &DECL_REPORT_MISSING,
            Self::ReportFound => &DECL_REPORT_FOUND,
            Self::EscortToSafety => &DECL_ESCORT_TO_SAFETY,
            Self::ProduceCommodity => &DECL_PRODUCE_COMMODITY,
            Self::SellCommodity => &DECL_SELL_COMMODITY,
            Self::RestockCommodity => &DECL_RESTOCK_COMMODITY,
            Self::MoveCargo => &DECL_MOVE_CARGO,
            Self::LootCorpse => &DECL_LOOT_CORPSE,
            Self::BuryCorpse => &DECL_BURY_CORPSE,
            Self::FulfillBounty => &DECL_FULFILL_BOUNTY,
            Self::PostBounty => &DECL_POST_BOUNTY,
            Self::PostNoticeWarning => &DECL_POST_NOTICE_WARNING,
            Self::PostNoticeOther => &DECL_POST_NOTICE_OTHER,
            Self::ShareBeliefAlarm => &DECL_SHARE_BELIEF_ALARM,
            Self::ShareBeliefTestimony => &DECL_SHARE_BELIEF_TESTIMONY,
            Self::ShareBeliefGossip => &DECL_SHARE_BELIEF_GOSSIP,
            Self::AskWitness => &DECL_ASK_WITNESS,
            Self::ClaimOffice => &DECL_CLAIM_OFFICE,
            Self::SupportCandidateForOffice => &DECL_SUPPORT_CANDIDATE_FOR_OFFICE,
            Self::InvestigateViolation => &DECL_INVESTIGATE_VIOLATION,
            Self::Patrol => &DECL_PATROL,
            Self::ExploreLocation => &DECL_EXPLORE_LOCATION,
            Self::StealItem => &DECL_STEAL_ITEM,
            Self::Accuse => &DECL_ACCUSE,
            Self::PunishFine => &DECL_PUNISH_FINE,
            Self::PunishExile => &DECL_PUNISH_EXILE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Authority, FeasibilityStrategy, FrontierExhaustionStrategy, GoalDispatchDeclaration,
        InvalidationStrategy, SELF_CARE_POLICY,
    };
    use crate::goal_policy::SuppressionRule;
    use crate::{GoalDispatchKey, GoalKindPlannerExt, PlannerOpKind};
    use worldwake_core::{
        AcquisitionQuantity, ArtifactPostingContext, BountyTarget, BountyTerms, CommodityKind,
        CommodityPurpose, EntityId, ExpectationId, GoalKind, HomeostaticNeedId, ProofRequirement,
        PunishmentKind, Quantity, RecipeId, RecordEntryId, RewardSource, TellTopic, ViolationId,
    };

    const ALL_KEYS: &[GoalDispatchKey] = &[
        GoalDispatchKey::ConsumeOwnedCommodity,
        GoalDispatchKey::AcquireSelfConsume,
        GoalDispatchKey::AcquireRecipeInput,
        GoalDispatchKey::AcquireRestock,
        GoalDispatchKey::Sleep,
        GoalDispatchKey::Relieve,
        GoalDispatchKey::Wash,
        GoalDispatchKey::FreeCarryCapacity,
        GoalDispatchKey::EngageHostile,
        GoalDispatchKey::RaidTarget,
        GoalDispatchKey::ReduceDanger,
        GoalDispatchKey::RegroupWithFaction,
        GoalDispatchKey::EstablishBanditCamp,
        GoalDispatchKey::TreatWounds,
        GoalDispatchKey::SearchForMissing,
        GoalDispatchKey::ReportMissing,
        GoalDispatchKey::ReportFound,
        GoalDispatchKey::EscortToSafety,
        GoalDispatchKey::ProduceCommodity,
        GoalDispatchKey::SellCommodity,
        GoalDispatchKey::RestockCommodity,
        GoalDispatchKey::MoveCargo,
        GoalDispatchKey::LootCorpse,
        GoalDispatchKey::BuryCorpse,
        GoalDispatchKey::FulfillBounty,
        GoalDispatchKey::PostBounty,
        GoalDispatchKey::PostNoticeWarning,
        GoalDispatchKey::PostNoticeOther,
        GoalDispatchKey::ShareBeliefAlarm,
        GoalDispatchKey::ShareBeliefTestimony,
        GoalDispatchKey::ShareBeliefGossip,
        GoalDispatchKey::AskWitness,
        GoalDispatchKey::ClaimOffice,
        GoalDispatchKey::SupportCandidateForOffice,
        GoalDispatchKey::InvestigateViolation,
        GoalDispatchKey::Patrol,
        GoalDispatchKey::ExploreLocation,
        GoalDispatchKey::StealItem,
        GoalDispatchKey::Accuse,
        GoalDispatchKey::PunishFine,
        GoalDispatchKey::PunishExile,
    ];

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn representative_goal_for(key: GoalDispatchKey) -> GoalKind {
        let target = entity(2);
        let office = entity(3);
        let destination = entity(4);

        match key {
            GoalDispatchKey::ConsumeOwnedCommodity => GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::AcquireSelfConsume => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            GoalDispatchKey::AcquireRecipeInput => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Grain,
                purpose: CommodityPurpose::RecipeInput(RecipeId(7)),
                quantity: AcquisitionQuantity::single(),
            },
            GoalDispatchKey::AcquireRestock => GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::Restock,
                quantity: AcquisitionQuantity::single(),
            },
            GoalDispatchKey::Sleep => GoalKind::Sleep,
            GoalDispatchKey::Relieve => GoalKind::Relieve,
            GoalDispatchKey::Wash => GoalKind::Wash,
            GoalDispatchKey::FreeCarryCapacity => GoalKind::FreeCarryCapacity,
            GoalDispatchKey::EngageHostile => GoalKind::EngageHostile { target },
            GoalDispatchKey::RaidTarget => GoalKind::RaidTarget { target },
            GoalDispatchKey::ReduceDanger => GoalKind::ReduceDanger,
            GoalDispatchKey::RegroupWithFaction => GoalKind::RegroupWithFaction { faction: office },
            GoalDispatchKey::EstablishBanditCamp => {
                GoalKind::EstablishBanditCamp { faction: office }
            }
            GoalDispatchKey::TreatWounds => GoalKind::TreatWounds { patient: target },
            GoalDispatchKey::SearchForMissing => GoalKind::SearchForMissing {
                subject: target,
                last_seen: Some(destination),
            },
            GoalDispatchKey::ReportMissing => GoalKind::ReportMissing {
                subject: target,
                to_office: Some(office),
                expectation_id: None,
            },
            GoalDispatchKey::ReportFound => GoalKind::ReportFound {
                subject: EntityId {
                    slot: 0,
                    generation: 0,
                },
                expectation_id: ExpectationId(0),
            },
            GoalDispatchKey::EscortToSafety => GoalKind::EscortToSafety {
                subject: target,
                destination,
            },
            GoalDispatchKey::ProduceCommodity => GoalKind::ProduceCommodity {
                recipe_id: RecipeId(11),
            },
            GoalDispatchKey::SellCommodity => GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::RestockCommodity => GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalDispatchKey::MoveCargo => GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            },
            GoalDispatchKey::LootCorpse => GoalKind::LootCorpse { corpse: target },
            GoalDispatchKey::BuryCorpse => GoalKind::BuryCorpse {
                corpse: target,
                burial_site: destination,
            },
            GoalDispatchKey::FulfillBounty => GoalKind::FulfillBounty { bounty: target },
            GoalDispatchKey::PostBounty => GoalKind::PostBounty {
                posting: ArtifactPostingContext {
                    posting_place: destination,
                    issuing_authority: None,
                    expires_at: None,
                    jurisdiction: None,
                },
                terms: BountyTerms {
                    target: BountyTarget::EliminateEntity { target },
                    proof_requirement: ProofRequirement::SelfReport,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(5),
                    reward_source: RewardSource::PersonalFunds { issuer: office },
                    claim_place: destination,
                },
            },
            GoalDispatchKey::PostNoticeWarning => GoalKind::PostNotice {
                posting: ArtifactPostingContext {
                    posting_place: destination,
                    issuing_authority: None,
                    expires_at: None,
                    jurisdiction: None,
                },
                topic: worldwake_core::NoticeTopic::ThreatWarning { place: destination },
            },
            GoalDispatchKey::PostNoticeOther => GoalKind::PostNotice {
                posting: ArtifactPostingContext {
                    posting_place: destination,
                    issuing_authority: None,
                    expires_at: None,
                    jurisdiction: None,
                },
                topic: worldwake_core::NoticeTopic::OfficeVacancy { office },
            },
            GoalDispatchKey::ShareBeliefAlarm => GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
                communication_class: worldwake_core::CommunicationClass::Alarm,
            },
            GoalDispatchKey::ShareBeliefTestimony => GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            },
            GoalDispatchKey::ShareBeliefGossip => GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            },
            GoalDispatchKey::AskWitness => GoalKind::AskWitness {
                witness: target,
                topic: TellTopic::EntityBelief { subject: office },
            },
            GoalDispatchKey::ClaimOffice => GoalKind::ClaimOffice { office },
            GoalDispatchKey::SupportCandidateForOffice => GoalKind::SupportCandidateForOffice {
                office,
                candidate: target,
            },
            GoalDispatchKey::InvestigateViolation => GoalKind::InvestigateViolation {
                violation_id: ViolationId(1),
                place: destination,
            },
            GoalDispatchKey::Patrol => GoalKind::Patrol { place: destination },
            GoalDispatchKey::ExploreLocation => GoalKind::ExploreLocation {
                target_place: destination,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    HomeostaticNeedId::Hunger,
                ),
                hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                    commodity: worldwake_core::CommodityKind::Apple,
                },
            },
            GoalDispatchKey::StealItem => GoalKind::StealItem {
                target_item: target,
            },
            GoalDispatchKey::Accuse => GoalKind::Accuse {
                crime_register: office,
                accused: target,
                violation_id: ViolationId(2),
            },
            GoalDispatchKey::PunishFine => GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(5),
                },
            },
            GoalDispatchKey::PunishExile => GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Exile {
                    from_faction: destination,
                },
            },
        }
    }

    #[test]
    fn test_declaration_completeness() {
        assert_eq!(ALL_KEYS.len(), 41);

        for key in ALL_KEYS {
            let declaration: &'static GoalDispatchDeclaration = key.declaration();
            assert!(!declaration.trace_label.is_empty());
        }
    }

    #[test]
    fn test_declaration_provenance_matches_live_goal_model() {
        for key in ALL_KEYS {
            let goal = representative_goal_for(*key);
            assert_eq!(
                key.declaration().provenance_family,
                goal.ranked_goal_provenance_family(),
                "provenance mismatch for {key:?}"
            );
        }
    }

    #[test]
    fn test_declaration_relevant_ops_match_live_goal_model() {
        for key in ALL_KEYS {
            let goal = representative_goal_for(*key);
            assert_eq!(
                key.declaration().relevant_ops,
                goal.relevant_op_kinds(),
                "relevant_ops mismatch for {key:?}"
            );
        }
    }

    #[test]
    fn test_relevant_ops_authority_is_hint_only_at_landing() {
        for key in ALL_KEYS {
            assert_eq!(
                key.declaration().relevant_ops_authority(),
                Authority::HintOnly,
                "relevant_ops authority mismatch for {key:?}"
            );
        }
    }

    #[test]
    fn test_patrol_declaration_uses_place_match_and_patrol_ops() {
        let declaration = GoalDispatchKey::Patrol.declaration();

        assert_eq!(declaration.trace_label, "Patrol");
        assert_eq!(
            declaration.relevant_ops,
            &[PlannerOpKind::Travel, PlannerOpKind::Patrol]
        );
        assert_eq!(
            declaration.invalidation_strategy,
            InvalidationStrategy::Patrol
        );
        assert_eq!(
            declaration.feasibility_strategy,
            FeasibilityStrategy::PlaceMatch
        );
    }

    #[test]
    fn test_free_carry_capacity_declaration_uses_drop_item_barrier_wiring() {
        let declaration = GoalDispatchKey::FreeCarryCapacity.declaration();

        assert_eq!(declaration.trace_label, "FreeCarryCapacity");
        assert_eq!(declaration.relevant_ops, &[PlannerOpKind::DropItem]);
        assert_eq!(
            declaration.invalidation_strategy,
            InvalidationStrategy::NoOpinion
        );
        assert_eq!(
            declaration.feasibility_strategy,
            FeasibilityStrategy::AlwaysLikely
        );
        assert_eq!(declaration.family_policy, SELF_CARE_POLICY);
        assert_eq!(declaration.progress_barrier_ops, &[PlannerOpKind::DropItem]);
    }

    #[test]
    fn ask_witness_declaration_uses_epistemic_sensing_policy() {
        let declaration = GoalDispatchKey::AskWitness.declaration();

        assert_eq!(declaration.trace_label, "AskWitness");
        assert_eq!(declaration.family_policy, super::EPISTEMIC_SENSING_POLICY);
        assert_eq!(
            declaration.progress_barrier_ops,
            &[PlannerOpKind::AskWitness]
        );
    }

    #[test]
    fn test_punish_fine_vs_exile_ops() {
        assert_ne!(
            GoalDispatchKey::PunishFine.declaration().relevant_ops,
            GoalDispatchKey::PunishExile.declaration().relevant_ops
        );
    }

    #[test]
    fn test_trace_labels_nonempty_and_distinct_for_payload_splits() {
        for key in ALL_KEYS {
            assert!(!key.declaration().trace_label.is_empty(), "{key:?}");
        }

        assert_ne!(
            GoalDispatchKey::AcquireSelfConsume
                .declaration()
                .trace_label,
            GoalDispatchKey::AcquireRecipeInput
                .declaration()
                .trace_label
        );
        assert_ne!(
            GoalDispatchKey::AcquireRecipeInput
                .declaration()
                .trace_label,
            GoalDispatchKey::AcquireRestock.declaration().trace_label
        );
        assert_ne!(
            GoalDispatchKey::PunishFine.declaration().trace_label,
            GoalDispatchKey::PunishExile.declaration().trace_label
        );
    }

    #[test]
    fn test_invalidation_strategies_cover_all_declarations() {
        for key in ALL_KEYS {
            let declaration = key.declaration();
            match declaration.invalidation_strategy {
                InvalidationStrategy::NoOpinion
                | InvalidationStrategy::CommodityOnly
                | InvalidationStrategy::AcquireCommodity
                | InvalidationStrategy::AcquireRestock
                | InvalidationStrategy::CombatTarget
                | InvalidationStrategy::DangerReduction
                | InvalidationStrategy::FactionRegroup
                | InvalidationStrategy::TreatWounds
                | InvalidationStrategy::ProduceCommodity
                | InvalidationStrategy::PositionAndCommodity
                | InvalidationStrategy::PositionCommodityAndCoin
                | InvalidationStrategy::PositionAndTargetDead
                | InvalidationStrategy::StealTargetState
                | InvalidationStrategy::ClaimOffice
                | InvalidationStrategy::SupportCandidateForOffice
                | InvalidationStrategy::InvestigateViolation
                | InvalidationStrategy::Patrol
                | InvalidationStrategy::BountyActive
                | InvalidationStrategy::PunishAccused => {}
                InvalidationStrategy::NeedWithFacilities(need)
                | InvalidationStrategy::NeedWithPosition(need) => {
                    assert!(
                        matches!(
                            need,
                            HomeostaticNeedId::Fatigue
                                | HomeostaticNeedId::Bladder
                                | HomeostaticNeedId::Dirtiness
                        ),
                        "{key:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_invalidation_strategies_match_payload_sensitive_and_shared_families() {
        assert_eq!(
            GoalDispatchKey::AcquireSelfConsume
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireCommodity
        );
        assert_eq!(
            GoalDispatchKey::AcquireRecipeInput
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireCommodity
        );
        assert_eq!(
            GoalDispatchKey::AcquireRestock
                .declaration()
                .invalidation_strategy,
            InvalidationStrategy::AcquireRestock
        );
        assert_eq!(
            GoalDispatchKey::LootCorpse
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::BuryCorpse
                .declaration()
                .invalidation_strategy
        );
        assert_eq!(
            GoalDispatchKey::SellCommodity
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::MoveCargo
                .declaration()
                .invalidation_strategy
        );
        assert_eq!(
            GoalDispatchKey::PunishFine
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::PunishExile
                .declaration()
                .invalidation_strategy
        );
        assert_ne!(
            GoalDispatchKey::RegroupWithFaction
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::ReduceDanger
                .declaration()
                .invalidation_strategy
        );
        assert_ne!(
            GoalDispatchKey::StealItem
                .declaration()
                .invalidation_strategy,
            GoalDispatchKey::LootCorpse
                .declaration()
                .invalidation_strategy
        );
    }

    #[test]
    fn frontier_exhaustion_strategy_covers_all_dispatch_declarations() {
        assert_eq!(GoalDispatchKey::all().len(), ALL_KEYS.len());

        for key in GoalDispatchKey::all() {
            match key.declaration().frontier_exhaustion_strategy {
                FrontierExhaustionStrategy::PermanentUntilInvalidator
                | FrontierExhaustionStrategy::CooldownRetry => {}
            }
        }
    }

    #[test]
    fn frontier_exhaustion_strategy_marks_recurring_retry_goals() {
        assert_eq!(
            GoalDispatchKey::Sleep
                .declaration()
                .frontier_exhaustion_strategy,
            FrontierExhaustionStrategy::CooldownRetry
        );
        assert_eq!(
            GoalDispatchKey::AcquireSelfConsume
                .declaration()
                .frontier_exhaustion_strategy,
            FrontierExhaustionStrategy::CooldownRetry
        );
        assert_eq!(
            GoalDispatchKey::Patrol
                .declaration()
                .frontier_exhaustion_strategy,
            FrontierExhaustionStrategy::CooldownRetry
        );
    }

    #[test]
    fn frontier_exhaustion_strategy_preserves_default_suppression_goals() {
        for key in [
            GoalDispatchKey::Wash,
            GoalDispatchKey::Relieve,
            GoalDispatchKey::AcquireRestock,
            GoalDispatchKey::ProduceCommodity,
        ] {
            assert_eq!(
                key.declaration().frontier_exhaustion_strategy,
                FrontierExhaustionStrategy::PermanentUntilInvalidator,
                "{key:?}"
            );
        }
    }

    #[test]
    fn test_feasibility_strategies_cover_all_declarations() {
        for key in ALL_KEYS {
            match key.declaration().feasibility_strategy {
                FeasibilityStrategy::OwnedCommodityCheck
                | FeasibilityStrategy::EvidencePlaceLocal
                | FeasibilityStrategy::AlwaysLikely
                | FeasibilityStrategy::CommodityPresenceCheck
                | FeasibilityStrategy::ColocationOrDead
                | FeasibilityStrategy::NoOpinion
                | FeasibilityStrategy::SellCheck
                | FeasibilityStrategy::CargoDestinationCheck
                | FeasibilityStrategy::CorpseBurialCheck
                | FeasibilityStrategy::PlaceMatch => {}
            }
        }
    }

    #[test]
    fn test_feasibility_strategies_match_payload_sensitive_and_shared_families() {
        assert_eq!(
            GoalDispatchKey::AcquireSelfConsume
                .declaration()
                .feasibility_strategy,
            FeasibilityStrategy::EvidencePlaceLocal
        );
        assert_eq!(
            GoalDispatchKey::AcquireRecipeInput
                .declaration()
                .feasibility_strategy,
            FeasibilityStrategy::EvidencePlaceLocal
        );
        assert_eq!(
            GoalDispatchKey::AcquireRestock
                .declaration()
                .feasibility_strategy,
            FeasibilityStrategy::EvidencePlaceLocal
        );
        assert_eq!(
            GoalDispatchKey::Sleep.declaration().feasibility_strategy,
            GoalDispatchKey::Relieve.declaration().feasibility_strategy
        );
        assert_eq!(
            GoalDispatchKey::EngageHostile
                .declaration()
                .feasibility_strategy,
            GoalDispatchKey::TreatWounds
                .declaration()
                .feasibility_strategy
        );
        assert_eq!(
            GoalDispatchKey::Accuse.declaration().feasibility_strategy,
            GoalDispatchKey::PunishFine
                .declaration()
                .feasibility_strategy
        );
        assert_eq!(
            GoalDispatchKey::PunishFine
                .declaration()
                .feasibility_strategy,
            GoalDispatchKey::PunishExile
                .declaration()
                .feasibility_strategy
        );
        assert_ne!(
            GoalDispatchKey::SellCommodity
                .declaration()
                .feasibility_strategy,
            GoalDispatchKey::MoveCargo
                .declaration()
                .feasibility_strategy
        );
        assert_ne!(
            GoalDispatchKey::LootCorpse
                .declaration()
                .feasibility_strategy,
            GoalDispatchKey::BuryCorpse
                .declaration()
                .feasibility_strategy
        );
    }

    #[test]
    fn sell_ops_contains_staff_market_not_trade() {
        let decl = GoalDispatchKey::SellCommodity.declaration();
        assert!(
            decl.relevant_ops.contains(&PlannerOpKind::StaffMarket),
            "SELL_OPS must contain StaffMarket"
        );
        assert!(
            !decl.relevant_ops.contains(&PlannerOpKind::Trade),
            "SELL_OPS must NOT contain Trade — seller goals plan through StaffMarket, not buyer-initiated Trade"
        );
        assert!(
            decl.relevant_ops.contains(&PlannerOpKind::Travel),
            "SELL_OPS must contain Travel"
        );
        assert!(
            decl.relevant_ops.contains(&PlannerOpKind::MoveCargo),
            "SELL_OPS must contain MoveCargo"
        );
        assert!(
            decl.relevant_ops.contains(&PlannerOpKind::StockManagement),
            "SELL_OPS must contain StockManagement"
        );
    }

    #[test]
    fn test_family_policy_declarations_cover_all_policy_variants() {
        use std::collections::HashSet;

        let mut suppression_variants = HashSet::new();
        let mut free_interrupt_variants = HashSet::new();

        for key in ALL_KEYS {
            let policy = key.declaration().family_policy;
            suppression_variants.insert(std::mem::discriminant(&policy.suppression));
            free_interrupt_variants.insert(std::mem::discriminant(&policy.free_interrupt));
        }

        // SuppressionRule has 2 variants: Never, WhenStressedAtOrAbove
        assert_eq!(
            suppression_variants.len(),
            2,
            "expected both SuppressionRule variants represented"
        );
        // FreeInterruptRole has 3 variants: Reactive, Opportunistic, Normal
        assert_eq!(
            free_interrupt_variants.len(),
            3,
            "expected all FreeInterruptRole variants represented"
        );
    }

    #[test]
    fn explore_location_uses_self_care_policy() {
        assert_eq!(
            GoalDispatchKey::ExploreLocation
                .declaration()
                .family_policy
                .suppression,
            SuppressionRule::Never,
            "ExploreLocation is a self-care fallback and must remain available under self-care stress"
        );
    }

    #[test]
    fn test_progress_barrier_ops_match_goal_model() {
        use crate::GoalKindPlannerExt;
        use crate::planner_ops::PlannedStep;
        use worldwake_core::ActionDefId;

        for key in ALL_KEYS {
            let goal = representative_goal_for(*key);
            let decl = key.declaration();

            for &op in decl.progress_barrier_ops {
                let step = PlannedStep {
                    def_id: ActionDefId(0),
                    targets: vec![],
                    target_place: None,
                    payload_override: None,
                    op_kind: op,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: vec![],
                    guard: None,
                    expectations: Vec::new(),
                };
                assert!(
                    goal.is_progress_barrier(&step),
                    "progress_barrier_ops contains {op:?} for {key:?}, but is_progress_barrier() returns false"
                );
            }
        }
    }
}
