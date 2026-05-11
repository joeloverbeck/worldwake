//! Structured decision trace data model and collection sink.
//!
//! Records per-agent per-tick decision reasoning for diagnostic
//! and test query purposes. See spec S08 for design rationale.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use worldwake_core::{
    ActionDefId, ActionDomain, ArtifactActionability, ArtifactCredibility, ArtifactExistence,
    ArtifactLegalEffect, ArtifactVisibility, BelievedArtifactState, BlockerKey, BlockingFact,
    CommodityKind, EntityId, FrameAssumption, FrameClearReason, GoalKey, HypothesisKind,
    InstitutionalClaim, InstitutionalKnowledgeSource, IntentionDomainTag, OmissionReason,
    OpportunityAnchor, OpportunityKey, PatrolRoute, PerceptionSource, Permille,
    PunishmentFineSelectionTrace, SuspensionReason, TellTopic, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionStartFailureReason, BindingStrictness, ResolvedRequestTrace,
    TellTopicOmissionReason,
};

use crate::ExhaustionRetryState;
use crate::agent_tick::portfolio::{FeasibilityVerdict, SlotKind};
use crate::feasibility::FeasibilityHint;
use crate::goal_model::{GoalPriorityClass, RankedGoalProvenance};
use crate::goal_switching::GoalSwitchKind;
use crate::interrupts::InterruptDecision;
use crate::knowledge_path::{
    BeliefAspect, BeliefProvenance, InstitutionalBeliefProvenance, KnowledgePath,
    SelfKnowledgeProvenance,
};
use crate::planner_duration_contract::PlannerDurationDependency;
use crate::planner_ops::{PlanTerminalKind, PlannerOpKind};
use crate::ranking::RankedGoalComparison;
use crate::side_benefit::SideBenefit;
use crate::source_composite::SourceCompositeRank;
// ── Frame Transition Trace ──────────────────────────────────────

/// One lifecycle event recorded for an `IntentionFrame` during a tick.
#[derive(Clone, Debug)]
pub enum FrameTransitionKind {
    Created {
        goal: GoalKey,
        domain_tag: IntentionDomainTag,
        patience_limit: u32,
        assumptions_count: usize,
    },
    Progressed {
        tick: Tick,
    },
    Suspended {
        reason: SuspensionReason,
        tick: Tick,
    },
    Resumed {
        tick: Tick,
    },
    Exhausted {
        stalled_ticks: u32,
        patience_limit: u32,
        blocked_intent_recorded: bool,
    },
    Cleared {
        reason: FrameClearReason,
        failed_assumption: Option<FrameAssumption>,
    },
}

/// Collected frame lifecycle events for one agent-tick.
#[derive(Clone, Debug)]
pub struct FrameTransitionTrace {
    pub transitions: Vec<FrameTransitionKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortfolioTrace {
    pub(crate) slots: BTreeMap<SlotKind, PortfolioSlotTrace>,
    pub(crate) slots_attempted: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortfolioSlotTrace {
    pub(crate) goal_key: GoalKey,
    pub(crate) motive_score: u32,
    pub(crate) feasibility: FeasibilityVerdict,
}

// ── Top-Level Record ────────────────────────────────────────────

/// One complete decision record for one agent at one tick.
#[derive(Clone, Debug)]
pub struct AgentDecisionTrace {
    pub agent: EntityId,
    pub tick: Tick,
    pub outcome: DecisionOutcome,
    pub opportunity_compiler_load: Option<OpportunityCompilerLoad>,
}

/// What the decision pipeline produced for this agent this tick.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DecisionOutcome {
    /// Agent is dead — no decision pipeline ran.
    Dead,

    /// Agent has an active action — interrupt evaluation ran.
    ActiveAction {
        action_def_id: ActionDefId,
        action_name: String,
        interrupt: InterruptTrace,
        frame_transition: Option<FrameTransitionTrace>,
    },

    /// Agent had no active action — full planning pipeline ran.
    Planning(Box<PlanningPipelineTrace>),
}

impl DecisionOutcome {
    /// One-line human-readable summary using stored strings only (no registry lookup).
    pub fn summary(&self) -> String {
        match self {
            DecisionOutcome::Dead => "DEAD — no decision".to_string(),
            DecisionOutcome::ActiveAction {
                action_name,
                interrupt,
                frame_transition,
                ..
            } => {
                let decision = &interrupt.decision;
                let ranking_suffix = interrupt
                    .top_challenger_comparison
                    .as_ref()
                    .map_or_else(String::new, format_ranked_goal_comparison_summary);
                let frame_suffix = format_frame_transition_summary(frame_transition.as_ref());
                format!(
                    "ACTIVE: {action_name} — interrupt: {decision:?}{ranking_suffix}{frame_suffix}"
                )
            }
            DecisionOutcome::Planning(planning) => {
                let selected = planning
                    .selection
                    .selected_goal()
                    .map_or_else(|| "none".to_string(), |goal| format_goal_key(&goal));
                let selected_opportunity = planning
                    .selection
                    .selected_opportunity
                    .map_or_else(|| "none".to_string(), format_opportunity_key);
                let selected_plan = planning
                    .selection
                    .selected_plan
                    .as_ref()
                    .map_or_else(|| "none".to_string(), format_selected_plan);
                let provenance = planning
                    .selection
                    .selected_plan_source
                    .as_ref()
                    .map_or_else(|| "none".to_string(), |source| format!("{source:?}"));
                let candidates = planning.candidates.ranked.len();
                let plans_found = planning
                    .planning
                    .attempts
                    .iter()
                    .filter(|a| matches!(a.outcome, PlanSearchOutcome::Found { .. }))
                    .count();
                let same_goal_suffix = planning
                    .planning
                    .same_goal_trace
                    .as_ref()
                    .map_or_else(String::new, format_same_goal_planning_trace_summary);
                let selected_summary = selected_ranked_goal_summary(planning);
                let selected_provenance = selected_summary
                    .and_then(|summary| summary.provenance.as_ref())
                    .map_or_else(String::new, format_ranked_goal_provenance_summary);
                let selected_feasibility = selected_summary
                    .map(|s| s.feasibility)
                    .filter(|f| *f != FeasibilityHint::Uncertain)
                    .map_or_else(String::new, |f| format!(", feasibility={f:?}"));
                let source_reliability_suffix = selected_summary
                    .and_then(|summary| summary.source_reliability_discount.as_ref())
                    .map_or_else(String::new, format_source_reliability_discount_summary);
                let source_composite_suffix = selected_summary
                    .and_then(|summary| summary.source_composite.as_ref())
                    .map_or_else(String::new, format_source_composite_summary);
                let competition_suffix = selected_summary
                    .and_then(|summary| summary.competition_discount.as_ref())
                    .map_or_else(String::new, format_competition_discount_summary);
                let acquisition_quantity_suffix = selected_summary
                    .and_then(|summary| summary.acquisition_quantity)
                    .map_or_else(String::new, format_acquisition_quantity_summary);
                let artifact_axis_suffix = selected_summary
                    .and_then(|summary| summary.artifact_axes.as_ref())
                    .map_or_else(String::new, format_artifact_axis_summary);
                let ranking_suffix = planning
                    .candidates
                    .top_ranked_comparison
                    .as_ref()
                    .map_or_else(String::new, format_ranked_goal_comparison_summary);
                let discrepancy_suffix = if planning.discrepancy_trace.is_empty() {
                    String::new()
                } else {
                    format!(", discrepancy_trace={}", planning.discrepancy_trace.len())
                };
                let replacement_suffix = planning
                    .selection
                    .plan_replacement
                    .as_ref()
                    .map_or_else(String::new, |replacement| {
                        format!(", replacement={:?}", replacement.kind)
                    });
                let frame_suffix =
                    format_frame_transition_summary(planning.frame_transition.as_ref());
                let patrol_suffix =
                    planning
                        .selected_patrol_anchor
                        .map_or_else(String::new, |anchor| {
                            format!(
                                ", patrol_waypoint={}, patrol_anchor={}",
                                planning
                                    .patrol_route
                                    .current_waypoint
                                    .map_or_else(|| "none".to_string(), |place| place.to_string()),
                                format_opportunity_anchor(anchor)
                            )
                        });
                let dirty = planning.dirty.display_names();
                format!(
                    "PLAN (dirty: {dirty}): selected={selected}, selected_opportunity={selected_opportunity}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}{same_goal_suffix}{replacement_suffix}{selected_provenance}{selected_feasibility}{source_reliability_suffix}{source_composite_suffix}{competition_suffix}{acquisition_quantity_suffix}{artifact_axis_suffix}{ranking_suffix}{discrepancy_suffix}{frame_suffix}{patrol_suffix}"
                )
            }
        }
    }
}

// ── Affordance Trace ────────────────────────────────────────────

/// Summary of one affordance available to the agent at decision time.
#[derive(Clone, Debug)]
pub struct AffordanceSummary {
    pub def_id: ActionDefId,
    pub action_name: String,
    pub target_count: usize,
}

/// Trace of all affordances available to the agent at the start of the
/// decision tick. This is the earliest causal input to the planning
/// pipeline — it determines which actions the planner can consider.
#[derive(Clone, Debug)]
pub struct AffordanceTrace {
    pub available: Vec<AffordanceSummary>,
    pub place: Option<EntityId>,
}

// ── Planning Pipeline ───────────────────────────────────────────

/// Full trace of the planning pipeline for one agent-tick.
#[derive(Clone, Debug)]
pub struct PlanningPipelineTrace {
    /// Affordances available to the agent at decision time.
    /// Populated only when tracing is enabled.
    pub affordances: Option<AffordanceTrace>,
    pub dirty: crate::DirtySet,
    /// When true, the existing plan was revalidated instead of replanning from
    /// scratch. This happens when `dirty.is_snapshot_only()` is true
    /// and the current plan's next step passes revalidation.
    pub plan_continued: bool,
    pub candidates: CandidateTrace,
    pub planning: PlanSearchTrace,
    pub selection: SelectionTrace,
    #[allow(dead_code)]
    pub portfolio: Option<PortfolioTrace>,
    pub execution: ExecutionTrace,
    /// Action start failures from the previous tick's `BestEffort` inputs,
    /// drained from the `Scheduler` for this agent.
    pub action_start_failures: Vec<ActionStartFailureSummary>,
    /// Active discrepancy-memory entries at trace construction time. Derived
    /// view for debuggability (P27).
    pub discrepancy_trace: Vec<DiscrepancyTrace>,
    /// Exhaustion cache state at trace construction time (P27).
    pub exhaustion_snapshot: Vec<ExhaustionTraceEntry>,
    /// Frame lifecycle events recorded during this tick (P27).
    pub frame_transition: Option<FrameTransitionTrace>,
    /// Planner-visible patrol-route snapshot used for patrol grounding on this tick.
    pub patrol_route: PatrolRouteSnapshotTrace,
    /// Selected patrol anchor when the chosen goal is a patrol branch.
    pub selected_patrol_anchor: Option<OpportunityAnchor>,
    /// When a pursuit plan was invalidated during the observation phase,
    /// records why. None when no pursuit invalidation occurred.
    pub pursuit_invalidation: Option<PursuitInvalidationReason>,
}

/// Debugger-facing snapshot of the actor's patrol-route state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PatrolRouteSnapshotTrace {
    pub route: Option<PatrolRoute>,
    pub current_waypoint: Option<EntityId>,
}

/// Summary of an action start failure for trace output.
#[derive(Clone, Debug)]
pub struct ActionStartFailureSummary {
    pub tick: Tick,
    pub def_id: ActionDefId,
    pub request: ResolvedRequestTrace,
    pub reason: ActionStartFailureReason,
}

/// Diagnostic trace for typed discrepancy entries active during planning.
/// Derived from `DiscrepancyMemory` at trace construction time (P27: derived view).
#[derive(Clone, Debug)]
pub struct DiscrepancyTrace {
    pub discrepancy: worldwake_core::Discrepancy,
    pub blocker_key: BlockerKey,
    pub expires_tick: Tick,
}

/// Snapshot of one opportunity's exhaustion state at trace time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExhaustionTraceEntry {
    pub opportunity: OpportunityKey,
    pub retry_state: ExhaustionRetryState,
    pub consecutive_failures: u8,
    pub next_retry_tick: Option<Tick>,
    pub retry_eligible: bool,
}

// ── Stage 1: Candidate Generation + Ranking ─────────────────────

/// Why an emitted candidate's score was reduced before ranking.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CandidateDampingReason {
    SurveyMemoryNegative {
        place: EntityId,
        hypothesis: HypothesisKind,
        recorded_tick: Tick,
        confidence: Permille,
    },
}

/// Diagnostic entry for a candidate that reached ranking with a reduced score.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateDampingEntry {
    pub goal_key: GoalKey,
    pub reason: CandidateDampingReason,
}

/// Trace of candidate generation and ranking.
#[derive(Clone, Debug, Default)]
pub struct CandidateTrace {
    /// All grounded goal keys generated (before suppression/zero-motive filter).
    pub generated: Vec<OpportunityKey>,
    /// Typed candidate-evidence provenance keyed by grounded goal.
    pub evidence: Vec<CandidateEvidenceTrace>,
    /// Desire-level diagnostic emitted when every concrete opportunity for a
    /// generated `GoalKey` was filtered out as blocked before ranking.
    pub fully_blocked_desires: Vec<DesireFullyBlocked>,
    /// Aggregate reachable-place count across acquisition-place searches.
    pub places_reachable: u32,
    /// Aggregate kept-place count after belief gating across acquisition-place searches.
    pub places_after_belief_filter: u32,
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoalSummary>,
    /// Why the highest-ranked goal beat the immediate runner-up, when at least
    /// two ranked candidates exist.
    pub top_ranked_comparison: Option<RankedGoalComparison>,
    /// Goals suppressed by situational conditions.
    pub suppressed: Vec<GoalKey>,
    /// Emitted goals whose ranking score was reduced by a soft damping reason.
    pub damped: Vec<CandidateDampingEntry>,
    /// Goals filtered by zero motive score.
    pub zero_motive: Vec<GoalKey>,
    /// Political goals omitted before generation due to hard gates.
    pub omitted_political: Vec<PoliticalCandidateOmission>,
    /// Bandit goals omitted before generation due to local candidate gates.
    pub omitted_bandit: Vec<BanditCandidateOmission>,
    /// Social goals omitted before generation due to resend suppression.
    pub omitted_social: Vec<SocialCandidateOmission>,
    /// Violation detection pass skipped due to missing prerequisites.
    pub omitted_violation_detection: Vec<ViolationDetectionOmission>,
}

impl CandidateTrace {
    #[must_use]
    pub fn generated_contains_goal(&self, goal_key: GoalKey) -> bool {
        self.generated
            .iter()
            .any(|opportunity| opportunity.goal_key == goal_key)
    }

    #[must_use]
    pub fn generated_contains_opportunity(&self, opportunity: OpportunityKey) -> bool {
        self.generated.contains(&opportunity)
    }

    #[must_use]
    pub fn ranked_summary_for_opportunity(
        &self,
        opportunity: OpportunityKey,
    ) -> Option<&RankedGoalSummary> {
        self.ranked
            .iter()
            .find(|summary| summary.opportunity == opportunity)
    }

    #[must_use]
    pub fn ranked_summaries_for_goal(&self, goal_key: GoalKey) -> Vec<&RankedGoalSummary> {
        self.ranked
            .iter()
            .filter(|summary| summary.opportunity.goal_key == goal_key)
            .collect()
    }

    #[must_use]
    pub fn evidence_for_opportunity(
        &self,
        opportunity: OpportunityKey,
    ) -> Option<&CandidateEvidenceTrace> {
        self.evidence
            .iter()
            .find(|trace| trace.opportunity == opportunity)
    }

    #[must_use]
    pub fn evidence_for_goal(&self, goal_key: GoalKey) -> Vec<&CandidateEvidenceTrace> {
        self.evidence
            .iter()
            .filter(|trace| trace.opportunity.goal_key == goal_key)
            .collect()
    }
}

/// Desire-level blocking summary for opportunity-scoped candidate filtering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesireFullyBlocked {
    pub goal_key: GoalKey,
    pub blocked_opportunities: Vec<OpportunityKey>,
    /// Per-opportunity blocker match details (parallel to `blocked_opportunities`).
    /// Empty when tracing is disabled.
    pub blocker_matches: Vec<BlockerMatchDetail>,
}

/// Records which blocker matched a specific filtered candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockerMatchDetail {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub expires_tick: Tick,
}

/// Political goal families that can be omitted before candidate emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoliticalGoalFamily {
    ClaimOffice,
    SupportCandidateForOffice,
    PostBounty,
}

/// Hard pre-emission reason for a political goal omission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoliticalCandidateOmissionReason {
    ForceSuccessionLaw,
    OfficeNotVisiblyVacant,
    OfficeHolderBeliefUnknownNoConsultableRecord,
    OfficeHolderBeliefConflicted,
    ActorNotEligible,
    CandidateNotEligible,
    AlreadyDeclaredSupport,
    SupportDeclarationBeliefConflicted,
    NoLawfulRewardSource,
}

/// Diagnostic record for a political goal omitted before generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoliticalCandidateOmission {
    pub family: PoliticalGoalFamily,
    pub office: EntityId,
    pub candidate: Option<EntityId>,
    pub reason: PoliticalCandidateOmissionReason,
}

/// Bandit goal families that can be omitted before candidate emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BanditGoalFamily {
    RegroupWithFaction,
    EstablishBanditCamp,
}

/// Hard pre-emission reason for a bandit goal omission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BanditCandidateOmissionReason {
    MissingRallyBelief,
    AlreadySafeInObservedActiveCamp,
    AlreadyAtRallyWithObservedActiveCamp,
    MissingLocalControlledEdibleSupplies,
}

/// Diagnostic record for a bandit goal omitted before generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BanditCandidateOmission {
    pub family: BanditGoalFamily,
    pub faction: EntityId,
    pub reason: BanditCandidateOmissionReason,
}

/// Diagnostic record for a social goal omitted before generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocialCandidateOmission {
    pub listener: EntityId,
    pub topic: TellTopic,
    pub reason: TellTopicOmissionReason,
}

/// Hard pre-emission reason for the entire violation-detection pass being skipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViolationDetectionOmissionReason {
    MissingViolationDispositionProfile,
    AgentInTransit,
}

/// Diagnostic record for violation detection skipped before candidate emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViolationDetectionOmission {
    pub reason: ViolationDetectionOmissionReason,
}

/// Summary of a ranked goal for trace output.
#[derive(Clone, Debug)]
pub struct RankedGoalSummary {
    pub opportunity: OpportunityKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    pub source_composite: Option<SourceCompositeRank>,
    pub feasibility: FeasibilityHint,
    /// Per-emission `AcquisitionQuantity` carried alongside the normalized
    /// goal identity. `Some` when the ranked goal is
    /// `GoalKind::AcquireCommodity`; `None` for all other goal families.
    /// Surfaces the per-agent `desired_min` / `desired_target` /
    /// `horizon_ticks` to the decision-trace pipeline (FND-29) without
    /// affecting `GoalKey` identity (S127 Design Goal 9).
    pub acquisition_quantity: Option<worldwake_core::AcquisitionQuantity>,
    /// Snapshot of the five social-artifact axes for an artifact referenced by
    /// this ranked candidate, when the candidate is grounded on one.
    pub artifact_axes: Option<ArtifactAxisSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactAxisSnapshot {
    pub artifact: EntityId,
    pub existence: ArtifactExistence,
    pub visibility: ArtifactVisibility,
    pub legal_effect: ArtifactLegalEffect,
    pub credibility: ArtifactCredibility,
    pub actionability: ArtifactActionability,
}

impl ArtifactAxisSnapshot {
    #[must_use]
    pub fn from_believed_artifact(artifact: EntityId, state: &BelievedArtifactState) -> Self {
        Self {
            artifact,
            existence: state.existence.clone(),
            visibility: state.visibility.clone(),
            legal_effect: state.legal_effect,
            credibility: state.credibility.clone(),
            actionability: state.actionability,
        }
    }
}

/// Records the competition discount applied to a ranked goal's motive score.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompetitionDiscount {
    pub observed_competitors: Vec<EntityId>,
    pub domain: ActionDomain,
    pub effective_discount: Permille,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
}

/// Records the source reliability discount applied to a ranked goal's motive score.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceReliabilityDiscount {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub failure_ratio_permille: u32,
    pub pre_discount_motive: u32,
    pub post_discount_motive: u32,
}

/// Actionable evidence contributor kind for one generated goal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateEvidenceKind {
    Seller,
    LooseLot,
    ResourceSource,
    Corpse,
    RecipeWorkstation,
    /// Listener in a Tell/ShareBelief interaction.
    Listener,
    /// Subject of a Tell/ShareBelief interaction.
    TellSubject,
    /// Office holder or candidate in political candidate generation.
    OfficeParticipant,
}

/// Why a candidate evidence contributor was excluded.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateEvidenceExclusionReason {
    DepletedResourceSource,
}

/// One actionable contributor that made a candidate emittable.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateEvidenceContributor {
    pub kind: CandidateEvidenceKind,
    pub place: EntityId,
    pub entity: EntityId,
}

/// One actionable contributor that was considered but excluded.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateEvidenceExclusion {
    pub kind: CandidateEvidenceKind,
    pub place: EntityId,
    pub entity: EntityId,
    pub reason: CandidateEvidenceExclusionReason,
}

/// Typed candidate-evidence provenance for one grounded goal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvidenceTrace {
    pub opportunity: OpportunityKey,
    pub contributors: Vec<CandidateEvidenceContributor>,
    pub exclusions: Vec<CandidateEvidenceExclusion>,
    /// Knowledge path: which beliefs motivated this candidate and where they came from.
    /// Empty when tracing is disabled.
    pub knowledge_path: KnowledgePath,
    /// Bounded legality provenance for goal families that need more than generic
    /// evidence / knowledge-path tracing.
    pub legality: Option<CandidateLegalityTrace>,
    /// Remote pursuit diagnostic: belief, confidence, route cost, and omission
    /// reason for pursuit candidates. Populated only when tracing is enabled.
    pub pursuit: Option<PursuitDiagnostic>,
    /// Snapshot of social-artifact axes for candidates grounded on an artifact.
    pub artifact_axes: Option<ArtifactAxisSnapshot>,
}

/// Goal-family-specific legality provenance for one generated candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateLegalityTrace {
    PunishmentFineSelection(PunishmentFineSelectionTrace),
}

/// Diagnostic provenance for a remote pursuit candidate (emitted or omitted).
///
/// Records the belief, confidence derivation, and route cost that drove or
/// prevented emission. Populated only when tracing is enabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PursuitDiagnostic {
    /// The target entity being pursued.
    pub target: EntityId,
    /// The place the agent believes the target occupies (None for `UnknownPlace` omission).
    pub believed_place: Option<EntityId>,
    /// How the agent learned the target's location (None for `UnknownPlace` omission).
    pub source: Option<PerceptionSource>,
    /// Tick when the target was observed at that place (None for `UnknownPlace` omission).
    pub observed_tick: Option<Tick>,
    /// Derived confidence value from `belief_confidence()` (None for `UnknownPlace` omission).
    pub derived_confidence: Option<Permille>,
    /// Profile threshold that confidence was compared against.
    pub min_confidence_threshold: Permille,
    /// Travel cost in ticks to the believed place (None if unreachable or not yet checked).
    pub route_cost: Option<u32>,
    /// Profile maximum pursuit travel ticks.
    pub max_travel_ticks: u32,
    /// If the candidate was omitted, the specific check that failed.
    pub omission: Option<PursuitOmissionReason>,
}

/// Why a remote pursuit candidate was omitted during candidate generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PursuitOmissionReason {
    /// `pursuit_target_belief()` returned None (unknown place, dead, or co-located).
    UnknownPlace,
    /// `believed_target_location()` marked the target-location belief as contradicted.
    ContradictedBelief,
    /// Derived confidence below `min_location_confidence`.
    LowConfidence,
    /// Route to believed place exceeds `max_pursuit_travel_ticks`.
    OverRange,
    /// No route exists to the believed place.
    Unreachable,
    /// Blocked by `BlockerMemory` for this target/place combination.
    Blocked,
}

/// Why an active remote pursuit plan was invalidated during the observation phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PursuitInvalidationReason {
    /// Agent no longer has a pursuit profile.
    NoProfile,
    /// No belief about the target entity.
    NoBelief,
    /// Target is believed dead.
    TargetDead,
    /// Target's believed place is unknown.
    PlaceUnknown,
    /// Target is now co-located (local combat, not remote pursuit).
    CoLocated,
    /// Believed place changed since the plan was formed.
    PlaceChanged,
    /// Derived confidence decayed below `min_location_confidence`.
    ConfidenceDecayed,
}

// ── Stage 2: Plan Search ────────────────────────────────────────

/// Trace of plan search attempts across candidates.
#[derive(Clone, Debug)]
pub struct PlanSearchTrace {
    /// One entry per candidate that was planned (top N by budget).
    pub attempts: Vec<PlanAttemptTrace>,
    /// Structured same-goal sibling continuation/stop provenance for this
    /// planning pass, when any candidates were admitted.
    pub same_goal_trace: Option<SameGoalPlanningTrace>,
}

/// Why same-goal sibling planning stopped after walking admitted candidates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SameGoalPlanningStopReason {
    EncounteredDifferentGoal { next_goal: GoalKey },
    ReachedCandidatePlanCap,
    ExhaustedAdmittedOpportunities,
}

/// Bounded provenance for same-goal sibling continuation and stop behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SameGoalPlanningTrace {
    pub continuation_trigger: Option<OpportunityKey>,
    pub stop_reason: SameGoalPlanningStopReason,
}

/// Diagnostic record of a candidate rejected by goal target binding.
#[derive(Clone, Debug)]
pub struct BindingRejection {
    pub def_id: ActionDefId,
    pub rejected_targets: Vec<EntityId>,
    pub required_target: Option<EntityId>,
}

/// Candidate payload provenance at the root search boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootCandidatePayloadStatus {
    None,
    CandidateProvided,
    GoalSynthesized,
}

/// Why a concrete root candidate was filtered before successor construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootCandidateFilterReason {
    BindingMismatch {
        required_target: Option<EntityId>,
    },
    CommodityIrrelevant {
        candidate_commodity: Option<CommodityKind>,
        goal_commodity: CommodityKind,
    },
    GoalUnavailable,
    BlockedFacilityUse {
        facility: EntityId,
        intended_action: ActionDefId,
    },
    PlaceBlocker {
        place: Option<EntityId>,
        blocking_fact: BlockingFact,
    },
    TravelCandidateCap {
        cap: u16,
    },
}

/// Why payload synthesis failed for a root candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadOverrideFailureReason {
    MissingTarget,
    UnsupportedGoal,
    MissingActorPlace,
    SellerUnavailable,
    SellerOutOfStock,
    ActorCannotPay,
}

/// Why a root candidate failed during successor construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootCandidateSkipReason {
    MissingActionDef,
    MissingSemantics,
    IrrelevantGoalOp,
    PayloadOverride(PayloadOverrideFailureReason),
    DurationEstimateFailed {
        dependency: PlannerDurationDependency,
    },
    HypotheticalTransitionFailed,
    NonTerminalLeafOnly,
    TotalDurationOverflow,
}

/// Final root-boundary status for one candidate seen by search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootCandidateOutcome {
    Expanded,
    Filtered(RootCandidateFilterReason),
    Skipped(RootCandidateSkipReason),
}

/// Structured root candidate provenance for one goal attempt.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CandidateSource {
    Emitter,
    OpportunityCompiler,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootCandidateTrace {
    pub def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: Option<PlannerOpKind>,
    pub authoritative_targets: Vec<EntityId>,
    pub planner_only: bool,
    pub payload_status: RootCandidatePayloadStatus,
    pub outcome: RootCandidateOutcome,
    pub omitted_anchor: Option<OmissionReason>,
    pub source: CandidateSource,
}

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct OpportunityCompilerLoad {
    pub compiled_count: u32,
    pub salience_floored: u32,
    pub learned_memory_damped: u32,
    pub cap_truncated: u32,
}

/// Final per-expansion status for one candidate after all pre-successor filters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionCandidateOutcome {
    Filtered(ExpansionCandidateFilterReason),
    Skipped(RootCandidateSkipReason),
    Terminal { terminal_kind: PlanTerminalKind },
    RetainedNonTerminal { preferred: bool },
    PrunedByBeam { preferred: bool },
}

/// Structured candidate provenance for one concrete expansion boundary after
/// root/tactical/travel filtering has completed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpansionCandidateTrace {
    pub def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: Option<PlannerOpKind>,
    pub authoritative_targets: Vec<EntityId>,
    pub planner_only: bool,
    pub payload_status: RootCandidatePayloadStatus,
    pub outcome: ExpansionCandidateOutcome,
}

/// Why a per-expansion candidate was filtered before successor construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionCandidateFilterReason {
    BindingMismatch {
        required_target: Option<EntityId>,
    },
    CommodityIrrelevant {
        candidate_commodity: Option<CommodityKind>,
        goal_commodity: CommodityKind,
    },
    GoalUnavailable,
    BlockedFacilityUse {
        facility: EntityId,
        intended_action: ActionDefId,
    },
    PlaceBlocker {
        place: Option<EntityId>,
        blocking_fact: BlockingFact,
    },
    TacticalGoalMismatch,
    TravelPrunedAwayFromGoal,
    TravelCandidateCap {
        cap: u16,
    },
}

/// Why a relevant root operator never produced a concrete root candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootOperatorOmissionReason {
    NoMatchingActionDef,
    NoAffordanceOrSynthesisPath,
    SynthesisUnsupportedGoalOp,
    SynthesisTargetDerivationFailed,
    ConditionalBarrierUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AskWitnessOmissionDetail {
    NoStaleEpistemicSubjects,
    NoWitnessAffordance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootOperatorOmissionDetail {
    AskWitness(AskWitnessOmissionDetail),
}

/// Structured omission provenance for one relevant operator at the root boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootOperatorOmissionTrace {
    pub op_kind: PlannerOpKind,
    pub reason: RootOperatorOmissionReason,
    pub detail: Option<RootOperatorOmissionDetail>,
}

/// Per-expansion summary recorded during plan search.
#[derive(Clone, Debug)]
pub struct SearchExpansionSummary {
    /// Depth (number of steps already in the node being expanded).
    pub depth: u8,
    /// Heuristic travel distance remaining from this node when spatial
    /// guidance is available; otherwise zero.
    pub remaining_travel_ticks: u32,
    /// Number of deduplicated places used to guide this expansion.
    pub combined_places_count: u16,
    /// Number of prerequisite-only places in the combined guidance set.
    pub prerequisite_places_count: u16,
    /// Total search candidates generated at this expansion.
    pub candidates_generated: u16,
    /// Candidates for which `build_successor` returned `None`.
    pub candidates_skipped: u16,
    /// Terminal successors found (`GoalSatisfied`, `ProgressBarrier`, `CombatCommitment`).
    pub terminal_successors: u16,
    /// Non-terminal successors before beam truncation.
    pub non_terminal_before_beam: u16,
    /// Non-terminal successors after beam truncation (pushed to frontier).
    pub non_terminal_after_beam: u16,
    /// Whether a `GoalSatisfied` terminal was found at this expansion
    /// (search returns immediately in this case).
    pub found_goal_satisfied: bool,
    /// Candidates marked as preferred by landmark guidance before beam truncation.
    pub preferred_candidates: u16,
    /// Current actionable-landmark count at this expansion boundary.
    pub landmark_heuristic: u32,
    /// The FF relaxed-plan heuristic value at this expansion boundary, or
    /// `None` when FF guidance was disabled or unavailable.
    pub ff_heuristic: Option<u32>,
    /// Number of helpful actions identified by FF guidance.
    pub helpful_action_count: u16,
    /// Travel-pruning facts captured before successor construction when the
    /// expansion had spatially guided travel choices.
    pub travel_pruning: Option<TravelPruningTrace>,
    /// Concrete goal-relevant / prerequisite guidance surfaces for this
    /// expansion boundary, when any exist.
    pub prerequisite_guidance: Option<PrerequisiteGuidanceTrace>,
    /// Candidate inventory that reached successor construction at this
    /// expansion boundary, including terminal, skipped, retained, and beam-pruned
    /// outcomes after all pre-successor filtering.
    pub expansion_candidates: Vec<ExpansionCandidateTrace>,
    /// Root candidate inventory and outcomes for this expansion. Populated only
    /// for the root expansion (`depth == 0`).
    pub root_candidates: Vec<RootCandidateTrace>,
    /// Relevant operators that never produced a root candidate. Populated only
    /// for the root expansion (`depth == 0`).
    pub root_omissions: Vec<RootOperatorOmissionTrace>,
}

/// Why a prerequisite place was excluded from guidance.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrerequisiteExclusionReason {
    DepletedResourceSource,
}

/// One prerequisite place excluded from guidance.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PrerequisiteExclusionTrace {
    pub place: EntityId,
    pub commodity: CommodityKind,
    pub reason: PrerequisiteExclusionReason,
}

/// Concrete guidance members used at one expansion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrerequisiteGuidanceTrace {
    pub goal_relevant_places: Vec<EntityId>,
    pub prerequisite_places: Vec<EntityId>,
    pub exclusions: Vec<PrerequisiteExclusionTrace>,
}

/// Remaining travel distance for one travel successor considered at an
/// expansion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TravelSuccessorTrace {
    pub destination: EntityId,
    pub base_ticks: u32,
    pub threat_permille: Permille,
    pub penalty_ticks: u32,
    pub direct_perceived_cost: u32,
    pub remaining_travel_ticks: u32,
    pub projected_total_cost: u32,
}

/// Structured summary of spatial pruning at one expansion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TravelPruningTrace {
    pub current_place: EntityId,
    pub current_remaining_travel_ticks: u32,
    pub retained: Vec<TravelSuccessorTrace>,
    pub pruned: Vec<TravelSuccessorTrace>,
}

/// Whether a failed-plan goal target was present in the actor's planning-time
/// known-entity inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetBeliefPresence {
    Present,
    Absent,
    NotApplicable,
}

/// Trace of a single plan search attempt for one goal.
#[derive(Clone, Debug)]
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub opportunity_anchor: OpportunityAnchor,
    pub outcome: PlanSearchOutcome,
    /// Strategic itinerary produced for this search attempt, when the planner
    /// entered the two-phase path and the itinerary had concrete steps.
    pub strategic_plan: Option<Vec<StrategicStepTrace>>,
    /// Active tactical barrier derived from the current strategic step, when present.
    pub tactical_goal: Option<String>,
    /// Count of fact landmarks extracted for this attempt.
    pub landmarks_extracted: u16,
    /// Count of landmark orderings extracted for this attempt.
    pub landmark_orderings: u16,
    /// Whether the actor had a planning-time belief entry for the goal's
    /// target entity. `NotApplicable` is used for targetless goals.
    pub target_belief_presence: TargetBeliefPresence,
    pub binding_rejections: Vec<BindingRejection>,
    /// Per-expansion summaries. Empty when tracing is disabled.
    pub expansion_summaries: Vec<SearchExpansionSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategicStepTrace {
    pub destination: EntityId,
    pub sub_goal: String,
    pub estimated_travel_ticks: u32,
}

/// Outcome of a plan search for one goal.
#[derive(Clone, Debug)]
pub enum PlanSearchOutcome {
    /// Plan found.
    Found {
        steps: Vec<PlannedStepSummary>,
        terminal_kind: PlanTerminalKind,
    },
    /// Node expansion budget exhausted.
    BudgetExhausted { expansions_used: u16 },
    /// Goal kind is unsupported by planner.
    Unsupported,
    /// Frontier exhausted without finding a plan.
    FrontierExhausted { expansions_used: u16 },
}

/// Summary of one planned step for trace output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedStepSummary {
    pub action_def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: PlannerOpKind,
    pub targets: Vec<EntityId>,
    pub estimated_ticks: u32,
    pub binding_strictness: Option<BindingStrictness>,
}

// ── Stage 3: Plan Selection ─────────────────────────────────────

/// Trace of plan selection and goal switching.
#[derive(Clone, Debug)]
pub struct SelectionTrace {
    /// The canonical opportunity identity for the selected plan, if one exists.
    pub selected_opportunity: Option<OpportunityKey>,
    /// Canonical summary of the final selected plan, if one exists.
    pub selected_plan: Option<SelectedPlanTrace>,
    /// Where the final selected plan came from.
    pub selected_plan_source: Option<SelectedPlanSource>,
    /// Whether a goal switch occurred from the previous tick's goal.
    pub goal_switch: Option<GoalSwitchSummary>,
    /// The previous goal (if any) for context.
    pub previous_goal: Option<GoalKey>,
    /// Explicit plan replacement summary when a fresh search displaces the
    /// current branch.
    pub plan_replacement: Option<SelectedPlanReplacementTrace>,
    /// Snapshot-continuation comparison summary when the planner evaluated
    /// whether to retain the current branch without a fresh search.
    pub snapshot_continuation: Option<SnapshotContinuationTrace>,
}

impl SelectionTrace {
    #[must_use]
    pub fn selected_goal(&self) -> Option<GoalKey> {
        self.selected_opportunity
            .map(|opportunity| opportunity.goal_key)
    }

    #[must_use]
    pub fn selected_goal_is(&self, goal_key: GoalKey) -> bool {
        self.selected_goal() == Some(goal_key)
    }

    #[must_use]
    pub fn selected_opportunity_is(&self, opportunity: OpportunityKey) -> bool {
        self.selected_opportunity == Some(opportunity)
    }
}

/// Canonical summary of the final plan the agent is following after selection.
#[derive(Clone, Debug)]
pub struct SelectedPlanTrace {
    pub steps: Vec<PlannedStepSummary>,
    pub terminal_kind: PlanTerminalKind,
    /// Step index the runtime will execute next, if any.
    pub next_step_index: Option<usize>,
    /// The next step on the selected path before execution/revalidation outcome.
    pub next_step: Option<PlannedStepSummary>,
    /// Compact summary of the winning fresh search when this selected plan came
    /// from `SearchSelection`.
    pub search_provenance: Option<SelectedPlanSearchProvenance>,
    /// Primary motive used for class/switch-margin comparisons.
    pub primary_motive: u32,
    /// Post-search bounded total including recognized side-benefits.
    pub total_value: u32,
    /// Secondary goals recognized along the selected plan's path.
    pub side_benefits: Vec<SideBenefitTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SideBenefitTrace {
    pub goal_key: GoalKey,
    pub at_place: EntityId,
    pub estimated_value: u32,
}

impl From<&SideBenefit> for SideBenefitTrace {
    fn from(value: &SideBenefit) -> Self {
        Self {
            goal_key: value.goal_key,
            at_place: value.at_place,
            estimated_value: value.estimated_value,
        }
    }
}

/// Compact planner-owned provenance for the selected fresh search result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPlanSearchProvenance {
    pub expansions_used: u16,
    pub root_remaining_travel_ticks: u32,
    pub root_travel_pruning: Option<TravelPruningTrace>,
    pub selected_root_travel_destination: Option<EntityId>,
}

/// Provenance for the final selected plan surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SelectedPlanSource {
    SearchSelection,
    RetainedCurrentPlan,
    SnapshotContinuation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotContinuationOutcome {
    ContinuedAsTopRanked,
    ContinuedWithinMargin,
    ReplannedHigherPriorityClass,
    ReplannedMarginExceeded,
    ReplannedCurrentOpportunityMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotContinuationTrace {
    pub current_opportunity: OpportunityKey,
    pub current_priority_class: Option<GoalPriorityClass>,
    pub current_motive_score: Option<u32>,
    pub top_opportunity: Option<OpportunityKey>,
    pub top_priority_class: Option<GoalPriorityClass>,
    pub top_motive_score: Option<u32>,
    pub planning_switch_margin: Permille,
    pub motive_delta: Option<u32>,
    pub outcome: SnapshotContinuationOutcome,
}

impl SnapshotContinuationTrace {
    #[must_use]
    pub const fn continues_plan(&self) -> bool {
        matches!(
            self.outcome,
            SnapshotContinuationOutcome::ContinuedAsTopRanked
                | SnapshotContinuationOutcome::ContinuedWithinMargin
        )
    }
}

/// Summary of a goal switch event.
#[derive(Clone, Debug)]
pub struct GoalSwitchSummary {
    pub from: GoalKey,
    pub to: GoalKey,
    pub kind: GoalSwitchKind,
}

/// How a fresh search replaced the current branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedPlanReplacementKind {
    SameGoalBranchRefreshed,
    SameGoalSiblingReplaced,
    GoalChanged,
}

/// Summary of a current-branch replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPlanReplacementTrace {
    pub previous_goal: GoalKey,
    pub new_goal: GoalKey,
    pub previous_next_step: Option<PlannedStepSummary>,
    pub new_next_step: Option<PlannedStepSummary>,
    pub kind: SelectedPlanReplacementKind,
}

// ── Stage 4: Execution Outcome ──────────────────────────────────

/// Trace of action execution attempt.
#[derive(Clone, Debug)]
pub struct ExecutionTrace {
    /// The step that was submitted for execution.
    pub enqueued_step: Option<PlannedStepSummary>,
    /// Whether revalidation of the step passed.
    pub revalidation_passed: Option<bool>,
    /// If the step could not be enqueued, why.
    pub failure: Option<ExecutionFailureReason>,
}

/// Why an execution attempt failed.
#[derive(Clone, Debug)]
pub enum ExecutionFailureReason {
    RevalidationFailed,
    TargetResolutionFailed,
    RecoverableTravelBlockage,
    PlanFailureHandled { blocked_goal: Option<GoalKey> },
}

// ── Interrupt Trace ─────────────────────────────────────────────

/// Trace of interrupt evaluation for an agent with an active action.
#[derive(Clone, Debug)]
pub struct InterruptTrace {
    pub decision: InterruptDecision,
    /// The highest-ranked challenger goal, if any.
    pub top_challenger: Option<RankedGoalSummary>,
    /// Why the highest-ranked challenger outranked the active goal at the
    /// ranking boundary, when both were present in the ranked candidate list.
    pub top_challenger_comparison: Option<RankedGoalComparison>,
}

/// Semantic status of one goal within one recorded agent tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalTraceStatus {
    NoTrace,
    Dead,
    ActiveAction,
    OmittedPolitical(PoliticalCandidateOmissionReason),
    OmittedBandit(BanditCandidateOmissionReason),
    OmittedSocial(TellTopicOmissionReason),
    NotGenerated,
    GeneratedOnly,
    Suppressed,
    ZeroMotive,
    Ranked { rank: usize, selected: bool },
}

impl GoalTraceStatus {
    #[must_use]
    pub fn is_generated(self) -> bool {
        matches!(
            self,
            Self::GeneratedOnly | Self::Suppressed | Self::ZeroMotive | Self::Ranked { .. }
        )
    }
}

/// Derived per-tick view of one goal's status and plan provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoalHistoryEntry {
    pub tick: Tick,
    pub status: GoalTraceStatus,
    pub plan_continued: bool,
    pub selected_plan_source: Option<SelectedPlanSource>,
}

impl PlanningPipelineTrace {
    #[must_use]
    pub fn selected_ranked_summary(&self) -> Option<&RankedGoalSummary> {
        let selected = self.selection.selected_opportunity?;
        self.candidates.ranked_summary_for_opportunity(selected)
    }
}

// ── Collection Sink ─────────────────────────────────────────────

/// Append-only collection of decision traces with query helpers.
///
/// All query methods compute on the fly from the internal `Vec` —
/// no derived state is stored.
#[derive(Clone, Debug)]
pub struct DecisionTraceSink {
    traces: Vec<AgentDecisionTrace>,
    opportunity_compiler_loads: BTreeMap<(EntityId, Tick), OpportunityCompilerLoad>,
}

impl DecisionTraceSink {
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
            opportunity_compiler_loads: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, trace: AgentDecisionTrace) {
        if let Some(load) = trace.opportunity_compiler_load {
            self.opportunity_compiler_loads
                .insert((trace.agent, trace.tick), load);
        }
        self.traces.push(trace);
    }

    pub fn record_opportunity_compiler_load(
        &mut self,
        agent: EntityId,
        tick: Tick,
        load: OpportunityCompilerLoad,
    ) {
        self.opportunity_compiler_loads.insert((agent, tick), load);
    }

    pub fn opportunity_compiler_load(
        &self,
        agent: EntityId,
        tick: Tick,
    ) -> Option<&OpportunityCompilerLoad> {
        self.opportunity_compiler_loads.get(&(agent, tick))
    }

    pub fn traces(&self) -> &[AgentDecisionTrace] {
        &self.traces
    }

    pub fn traces_for(&self, agent: EntityId) -> Vec<&AgentDecisionTrace> {
        self.traces.iter().filter(|t| t.agent == agent).collect()
    }

    pub fn trace_at(&self, agent: EntityId, tick: Tick) -> Option<&AgentDecisionTrace> {
        self.traces
            .iter()
            .find(|t| t.agent == agent && t.tick == tick)
    }

    #[must_use]
    pub fn goal_status_at(
        &self,
        agent: EntityId,
        tick: Tick,
        goal: &crate::GoalKind,
    ) -> GoalTraceStatus {
        self.trace_at(agent, tick)
            .map_or(GoalTraceStatus::NoTrace, |trace| trace.goal_status(goal))
    }

    #[must_use]
    pub fn goal_history_for(
        &self,
        agent: EntityId,
        goal: &crate::GoalKind,
    ) -> Vec<GoalHistoryEntry> {
        self.traces_for(agent)
            .into_iter()
            .map(|trace| trace.goal_history_entry(goal))
            .collect()
    }

    pub fn clear(&mut self) {
        self.traces.clear();
        self.opportunity_compiler_loads.clear();
    }

    /// Print a human-readable summary for one agent across all recorded ticks.
    ///
    /// Output goes to stderr for interactive debugging. This method never panics
    /// regardless of trace contents.
    pub fn dump_agent(&self, agent: EntityId, action_defs: &ActionDefRegistry) {
        for trace in self.traces_for(agent) {
            eprintln!(
                "[tick {}] {}",
                trace.tick.0,
                format_outcome(&trace.outcome, action_defs)
            );
            // Render per-candidate evidence and knowledge paths for Planning outcomes.
            if let DecisionOutcome::Planning(ref planning) = trace.outcome {
                for ranked in &planning.candidates.ranked {
                    if ranked.source_reliability_discount.is_some()
                        || ranked.source_composite.is_some()
                        || ranked.competition_discount.is_some()
                        || ranked.artifact_axes.is_some()
                    {
                        let source_reliability_suffix = ranked
                            .source_reliability_discount
                            .as_ref()
                            .map_or_else(String::new, format_source_reliability_discount_summary);
                        let source_composite_suffix = ranked
                            .source_composite
                            .as_ref()
                            .map_or_else(String::new, format_source_composite_summary);
                        let competition_suffix = ranked
                            .competition_discount
                            .as_ref()
                            .map_or_else(String::new, format_competition_discount_summary);
                        let artifact_axis_suffix = ranked
                            .artifact_axes
                            .as_ref()
                            .map_or_else(String::new, format_artifact_axis_summary);
                        eprintln!(
                            "  Ranked: {}{}{}{}{}",
                            format_opportunity_key(ranked.opportunity),
                            source_reliability_suffix,
                            source_composite_suffix,
                            competition_suffix,
                            artifact_axis_suffix
                        );
                    }
                }
                for ev in &planning.candidates.evidence {
                    let feasibility = planning
                        .candidates
                        .ranked
                        .iter()
                        .find(|r| r.opportunity == ev.opportunity)
                        .map_or(FeasibilityHint::Uncertain, |r| r.feasibility);
                    eprintln!(
                        "  Candidate: {} [feasibility={feasibility:?}]",
                        format_opportunity_key(ev.opportunity)
                    );
                    if !ev.contributors.is_empty() {
                        let contrib_strs: Vec<String> = ev
                            .contributors
                            .iter()
                            .map(|c| format!("{:?}({:?} @ {:?})", c.kind, c.entity, c.place))
                            .collect();
                        eprintln!("    Evidence: {}", contrib_strs.join(", "));
                    }
                    if !ev.exclusions.is_empty() {
                        let excl_strs: Vec<String> = ev
                            .exclusions
                            .iter()
                            .map(|e| {
                                format!(
                                    "{:?}({:?} @ {:?}) reason={:?}",
                                    e.kind, e.entity, e.place, e.reason
                                )
                            })
                            .collect();
                        eprintln!("    Exclusions: {}", excl_strs.join(", "));
                    }
                    for line in format_knowledge_path(&ev.knowledge_path) {
                        eprintln!("{line}");
                    }
                    if let Some(ref pd) = ev.pursuit {
                        format_pursuit_diagnostic(pd);
                    }
                }
                for omission in &planning.candidates.omitted_violation_detection {
                    eprintln!("  Violation detection skipped: {:?}", omission.reason);
                }
                for damping in &planning.candidates.damped {
                    eprintln!("  {}", format_candidate_damping_entry(damping));
                }
                if let Some(reason) = planning.pursuit_invalidation {
                    eprintln!("  Pursuit invalidated: {reason:?}");
                }
            }
        }
    }
}

fn format_pursuit_diagnostic(pd: &PursuitDiagnostic) {
    if let Some(omission) = pd.omission {
        eprintln!(
            "    Pursuit OMITTED: target={:?} reason={:?}",
            pd.target, omission
        );
    } else {
        eprintln!("    Pursuit EMITTED: target={:?}", pd.target);
    }
    if let Some(place) = pd.believed_place {
        eprintln!("      believed_place={place:?}");
    }
    if let (Some(source), Some(tick)) = (pd.source, pd.observed_tick) {
        eprintln!("      source={source:?} observed_tick={}", tick.0);
    }
    if let Some(conf) = pd.derived_confidence {
        eprintln!(
            "      confidence={}/{} (min={})",
            conf.value(),
            1000,
            pd.min_confidence_threshold.value()
        );
    }
    if let Some(cost) = pd.route_cost {
        eprintln!(
            "      route_cost={}/{} (max={})",
            cost, pd.max_travel_ticks, pd.max_travel_ticks
        );
    }
}

impl AgentDecisionTrace {
    #[must_use]
    pub fn goal_status(&self, goal: &crate::GoalKind) -> GoalTraceStatus {
        match &self.outcome {
            DecisionOutcome::Dead => GoalTraceStatus::Dead,
            DecisionOutcome::ActiveAction { .. } => GoalTraceStatus::ActiveAction,
            DecisionOutcome::Planning(planning) => goal_status_in_planning(planning, goal),
        }
    }

    #[must_use]
    pub fn goal_history_entry(&self, goal: &crate::GoalKind) -> GoalHistoryEntry {
        match &self.outcome {
            DecisionOutcome::Planning(planning) => GoalHistoryEntry {
                tick: self.tick,
                status: goal_status_in_planning(planning, goal),
                plan_continued: planning.plan_continued,
                selected_plan_source: planning.selection.selected_plan_source,
            },
            _ => GoalHistoryEntry {
                tick: self.tick,
                status: self.goal_status(goal),
                plan_continued: false,
                selected_plan_source: None,
            },
        }
    }
}

fn goal_status_in_planning(
    planning: &PlanningPipelineTrace,
    goal: &crate::GoalKind,
) -> GoalTraceStatus {
    if let Some(reason) =
        omitted_political_reason_for_goal(&planning.candidates.omitted_political, goal)
    {
        return GoalTraceStatus::OmittedPolitical(reason);
    }
    if let Some(reason) = omitted_bandit_reason_for_goal(&planning.candidates.omitted_bandit, goal)
    {
        return GoalTraceStatus::OmittedBandit(reason);
    }
    if let Some(reason) = omitted_social_reason_for_goal(&planning.candidates.omitted_social, goal)
    {
        return GoalTraceStatus::OmittedSocial(reason);
    }

    let goal_key = GoalKey::from(goal);
    if planning.candidates.suppressed.contains(&goal_key) {
        return GoalTraceStatus::Suppressed;
    }
    if planning.candidates.zero_motive.contains(&goal_key) {
        return GoalTraceStatus::ZeroMotive;
    }
    if let Some(rank) = planning
        .candidates
        .ranked_summaries_for_goal(goal_key)
        .into_iter()
        .next()
        .and_then(|summary| {
            planning
                .candidates
                .ranked
                .iter()
                .position(|candidate| candidate.opportunity == summary.opportunity)
        })
    {
        return GoalTraceStatus::Ranked {
            rank,
            selected: planning.selection.selected_goal_is(goal_key),
        };
    }
    if planning.candidates.generated_contains_goal(goal_key) {
        return GoalTraceStatus::GeneratedOnly;
    }
    GoalTraceStatus::NotGenerated
}

fn omitted_political_reason_for_goal(
    omissions: &[PoliticalCandidateOmission],
    goal: &crate::GoalKind,
) -> Option<PoliticalCandidateOmissionReason> {
    omissions.iter().find_map(|omission| match goal {
        crate::GoalKind::ClaimOffice { office }
            if omission.family == PoliticalGoalFamily::ClaimOffice
                && omission.office == *office
                && omission.candidate.is_none() =>
        {
            Some(omission.reason)
        }
        crate::GoalKind::SupportCandidateForOffice { office, candidate }
            if omission.family == PoliticalGoalFamily::SupportCandidateForOffice
                && omission.office == *office
                && omission.candidate == Some(*candidate) =>
        {
            Some(omission.reason)
        }
        crate::GoalKind::PostBounty { posting, terms }
            if omission.family == PoliticalGoalFamily::PostBounty
                && posting.issuing_authority == Some(omission.office)
                && matches!(
                    terms.target,
                    worldwake_core::BountyTarget::EliminateEntity { target }
                        if omission.candidate == Some(target)
                ) =>
        {
            Some(omission.reason)
        }
        // Political omissions are only recorded for political goal families.
        // Non-political goals correctly have no matching omission reason here.
        _ => None,
    })
}

fn omitted_bandit_reason_for_goal(
    omissions: &[BanditCandidateOmission],
    goal: &crate::GoalKind,
) -> Option<BanditCandidateOmissionReason> {
    omissions.iter().find_map(|omission| match goal {
        crate::GoalKind::RegroupWithFaction { faction }
            if omission.family == BanditGoalFamily::RegroupWithFaction
                && omission.faction == *faction =>
        {
            Some(omission.reason)
        }
        crate::GoalKind::EstablishBanditCamp { faction }
            if omission.family == BanditGoalFamily::EstablishBanditCamp
                && omission.faction == *faction =>
        {
            Some(omission.reason)
        }
        _ => None,
    })
}

fn omitted_social_reason_for_goal(
    omissions: &[SocialCandidateOmission],
    goal: &crate::GoalKind,
) -> Option<TellTopicOmissionReason> {
    omissions.iter().find_map(|omission| match goal {
        crate::GoalKind::ShareBelief {
            listener, topic, ..
        } if omission.listener == *listener && omission.topic == *topic => Some(omission.reason),
        // Social omissions are only recorded for ShareBelief candidates.
        // Other goal families correctly fall through with no omission reason.
        _ => None,
    })
}

/// Format a `DecisionOutcome` with action name resolution via the registry.
#[allow(clippy::too_many_lines)]
fn format_outcome(outcome: &DecisionOutcome, action_defs: &ActionDefRegistry) -> String {
    match outcome {
        DecisionOutcome::Dead => "DEAD — no decision".to_string(),
        DecisionOutcome::ActiveAction {
            action_def_id,
            action_name,
            interrupt,
            frame_transition,
            ..
        } => {
            let name = action_defs
                .get(*action_def_id)
                .map_or(action_name.as_str(), |d| d.name.as_str());
            let decision = &interrupt.decision;
            let challenger = interrupt
                .top_challenger
                .as_ref()
                .and_then(|summary| summary.provenance.as_ref())
                .map_or_else(String::new, format_ranked_goal_provenance_summary);
            let ranking = interrupt
                .top_challenger_comparison
                .as_ref()
                .map_or_else(String::new, format_ranked_goal_comparison_summary);
            let frame_suffix = format_frame_transition_summary(frame_transition.as_ref());
            format!("ACTIVE: {name} — interrupt: {decision:?}{challenger}{ranking}{frame_suffix}")
        }
        DecisionOutcome::Planning(planning) => {
            let selected = planning
                .selection
                .selected_goal()
                .map_or_else(|| "none".to_string(), |goal| format_goal_key(&goal));
            let selected_plan = planning
                .selection
                .selected_plan
                .as_ref()
                .map_or_else(|| "none".to_string(), format_selected_plan);
            let provenance = planning
                .selection
                .selected_plan_source
                .as_ref()
                .map_or_else(|| "none".to_string(), |source| format!("{source:?}"));
            let candidates = planning.candidates.ranked.len();
            let plans_found = planning
                .planning
                .attempts
                .iter()
                .filter(|a| matches!(a.outcome, PlanSearchOutcome::Found { .. }))
                .count();
            let selected_summary = selected_ranked_goal_summary(planning);
            let selected_provenance = selected_summary
                .and_then(|summary| summary.provenance.as_ref())
                .map_or_else(String::new, format_ranked_goal_provenance_summary);
            let selected_feasibility = selected_summary
                .map(|s| s.feasibility)
                .filter(|f| *f != FeasibilityHint::Uncertain)
                .map_or_else(String::new, |f| format!(", feasibility={f:?}"));
            let competition = selected_summary
                .and_then(|summary| summary.competition_discount.as_ref())
                .map_or_else(String::new, format_competition_discount_summary);
            let acquisition_quantity_suffix = selected_summary
                .and_then(|summary| summary.acquisition_quantity)
                .map_or_else(String::new, format_acquisition_quantity_summary);
            let artifact_axis_suffix = selected_summary
                .and_then(|summary| summary.artifact_axes.as_ref())
                .map_or_else(String::new, format_artifact_axis_summary);
            let ranking = planning
                .candidates
                .top_ranked_comparison
                .as_ref()
                .map_or_else(String::new, format_ranked_goal_comparison_summary);
            let dirty = planning.dirty.display_names();
            let mut out = format!(
                "PLAN (dirty: {dirty}): selected={selected}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}{selected_provenance}{selected_feasibility}{competition}{acquisition_quantity_suffix}{artifact_axis_suffix}{ranking}"
            );
            if let Some(ref aff) = planning.affordances {
                let place_str = aff
                    .place
                    .map_or_else(|| "none".to_string(), |p| format!("{p:?}"));
                let _ = write!(out, "\n  Place: {place_str}");
                let names: Vec<String> = aff
                    .available
                    .iter()
                    .map(|a| {
                        if a.target_count > 0 {
                            format!("{}({} targets)", a.action_name, a.target_count)
                        } else {
                            a.action_name.clone()
                        }
                    })
                    .collect();
                let _ = write!(out, "\n  Affordances: [{}]", names.join(", "));
            }
            for blocked in &planning.candidates.fully_blocked_desires {
                let _ = write!(
                    out,
                    "\n  fully blocked desire: goal={}, opportunities={:?}",
                    format_goal_key(&blocked.goal_key),
                    blocked.blocked_opportunities
                );
                for detail in &blocked.blocker_matches {
                    let action_name = detail
                        .blocker_key
                        .action_def
                        .and_then(|id| action_defs.get(id))
                        .map_or("none".to_string(), |d| d.name.clone());
                    let _ = write!(
                        out,
                        "\n    blocker: goal={}, place={:?}, target={:?}, action={}, fact={:?}, expires={}",
                        format_goal_key(&detail.blocker_key.goal_key),
                        detail.blocker_key.place,
                        detail.blocker_key.target,
                        action_name,
                        detail.blocking_fact,
                        detail.expires_tick.0,
                    );
                }
            }
            for attempt in &planning.planning.attempts {
                let _ = write!(
                    out,
                    "\n  plan attempt: goal={}, anchor={:?}, outcome={:?}",
                    format_goal_key(&attempt.goal),
                    attempt.opportunity_anchor,
                    attempt.outcome
                );
                for rej in &attempt.binding_rejections {
                    let def_name = action_defs
                        .get(rej.def_id)
                        .map_or("unknown", |d| d.name.as_str());
                    let _ = write!(
                        out,
                        "\n  binding rejected: {def_name} targets={:?} required={:?}",
                        rej.rejected_targets, rej.required_target
                    );
                }
                if let Some(steps) = &attempt.strategic_plan {
                    let _ = write!(
                        out,
                        "\n  strategic plan: {} steps, {} landmarks, {} orderings",
                        steps.len(),
                        attempt.landmarks_extracted,
                        attempt.landmark_orderings
                    );
                    for step in steps {
                        let _ = write!(
                            out,
                            "\n    strategic step: dest={:?} sub_goal={} travel={}",
                            step.destination, step.sub_goal, step.estimated_travel_ticks
                        );
                    }
                }
                if let Some(tactical_goal) = &attempt.tactical_goal {
                    let _ = write!(out, "\n  tactical goal: {tactical_goal}");
                }
                for exp in &attempt.expansion_summaries {
                    let satisfied_tag = if exp.found_goal_satisfied {
                        " satisfied"
                    } else {
                        ""
                    };
                    let ff_suffix = exp.ff_heuristic.map_or_else(String::new, |heuristic| {
                        format!(
                            ", h_ff={heuristic}, helpful_actions={}",
                            exp.helpful_action_count
                        )
                    });
                    let _ = write!(
                        out,
                        "\n  search expansion d={}: {} candidates, {} preferred, h_landmark={}{} , {} skipped, {} terminal{}, {}→{} beam",
                        exp.depth,
                        exp.candidates_generated,
                        exp.preferred_candidates,
                        exp.landmark_heuristic,
                        ff_suffix,
                        exp.candidates_skipped,
                        exp.terminal_successors,
                        satisfied_tag,
                        exp.non_terminal_before_beam,
                        exp.non_terminal_after_beam,
                    );
                    for omission in &exp.root_omissions {
                        let _ = write!(
                            out,
                            "\n    root omission: {:?} -> {:?}",
                            omission.op_kind, omission.reason
                        );
                    }
                    for candidate in &exp.root_candidates {
                        let op_kind = candidate
                            .op_kind
                            .map_or_else(|| "none".to_string(), |op| format!("{op:?}"));
                        let _ = write!(
                            out,
                            "\n    root candidate: {} op={} payload={:?} outcome={:?}",
                            candidate.action_name,
                            op_kind,
                            candidate.payload_status,
                            candidate.outcome
                        );
                    }
                }
            }
            if !planning.discrepancy_trace.is_empty() {
                let _ = write!(out, "\n  Discrepancies active:");
                for discrepancy in &planning.discrepancy_trace {
                    let def_name = discrepancy
                        .blocker_key
                        .action_def
                        .and_then(|action_def| action_defs.get(action_def))
                        .map_or("unknown", |d| d.name.as_str());
                    let _ = write!(
                        out,
                        "\n    goal={} discrepancy={:?} action={def_name} place={:?} expires_tick={}",
                        format_goal_key(&discrepancy.blocker_key.goal_key),
                        discrepancy.discrepancy,
                        discrepancy.blocker_key.place,
                        discrepancy.expires_tick.0,
                    );
                }
            }
            if let Some(ref ft) = planning.frame_transition {
                let _ = write!(out, "\n  Frame transitions:");
                for t in &ft.transitions {
                    let _ = write!(out, "\n    {}", format_frame_transition_kind(t));
                }
            }
            out
        }
    }
}

fn selected_ranked_goal_summary(planning: &PlanningPipelineTrace) -> Option<&RankedGoalSummary> {
    planning.selected_ranked_summary()
}

fn format_goal_kind(goal: &crate::GoalKind) -> String {
    let label = crate::GoalDispatchKey::from_goal_kind(goal)
        .declaration()
        .trace_label;
    let detail = format!("{goal:?}");
    if detail == label {
        label.to_string()
    } else {
        format!("{label} [{detail}]")
    }
}

fn format_goal_key(goal: &GoalKey) -> String {
    format_goal_kind(&goal.kind)
}

fn format_candidate_damping_entry(entry: &CandidateDampingEntry) -> String {
    match &entry.reason {
        CandidateDampingReason::SurveyMemoryNegative {
            place,
            hypothesis,
            recorded_tick,
            confidence,
        } => format!(
            "{} damped by SurveyMemory: found=false at tick {}, confidence={}.",
            format_damped_goal_key(&entry.goal_key, *place, *hypothesis),
            recorded_tick.0,
            confidence.value()
        ),
    }
}

fn format_damped_goal_key(goal: &GoalKey, place: EntityId, hypothesis: HypothesisKind) -> String {
    match goal.kind {
        crate::GoalKind::ExploreLocation { .. } => {
            format!("ExploreLocation {{ target: {place}, hypothesis: {hypothesis:?} }}")
        }
        _ => format_goal_key(goal),
    }
}

fn format_opportunity_key(opportunity: OpportunityKey) -> String {
    format!(
        "{}@{}",
        format_goal_key(&opportunity.goal_key),
        format_opportunity_anchor(opportunity.anchor)
    )
}

fn format_opportunity_anchor(anchor: OpportunityAnchor) -> String {
    match anchor {
        OpportunityAnchor::Place(place) => format!("place:{place}"),
        OpportunityAnchor::Entity(entity) => format!("entity:{entity}"),
        OpportunityAnchor::None => "none".to_string(),
    }
}

fn format_ranked_goal_provenance_summary(provenance: &RankedGoalProvenance) -> String {
    match provenance {
        RankedGoalProvenance::Danger(assessment) => format!(
            ", danger=pressure={} attackers={:?} visible_hostiles={:?} hostile_targets={:?} wounds={} incapacitated={}",
            assessment.pressure.value(),
            assessment.current_attackers,
            assessment.visible_hostiles,
            assessment.hostile_targets,
            assessment.has_wounds,
            assessment.is_incapacitated,
        ),
        RankedGoalProvenance::Drive(provenance) => {
            let motive_inputs = provenance
                .motive_inputs
                .iter()
                .map(|input| {
                    format!(
                        "{:?}(pressure={}, weight={}, score={}, recovery_relevant={})",
                        input.drive,
                        input.pressure.value(),
                        input.weight.value(),
                        input.score,
                        input.recovery_relevant,
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let adjustment = provenance.adjustment.map_or_else(
                || "none".to_string(),
                |adjustment| format!("{adjustment:?}"),
            );
            format!(
                ", drive=base={:?} final={:?} adjustment={} motive_inputs=[{}]",
                provenance.base_priority_class,
                provenance.final_priority_class,
                adjustment,
                motive_inputs,
            )
        }
    }
}

fn format_ranked_goal_comparison_summary(comparison: &RankedGoalComparison) -> String {
    format!(
        ", ranking={:?} {}>{}",
        comparison.decisive_dimension,
        format_opportunity_key(comparison.winner),
        format_opportunity_key(comparison.loser)
    )
}

fn format_acquisition_quantity_summary(quantity: worldwake_core::AcquisitionQuantity) -> String {
    format!(
        ", acquisition=desired_min={} desired_target={} horizon_ticks={}",
        quantity.desired_min.get(),
        quantity.desired_target.get(),
        quantity.horizon_ticks.get(),
    )
}

fn format_artifact_axis_summary(snapshot: &ArtifactAxisSnapshot) -> String {
    format!(
        ", artifact_axes=artifact={:?} existence={:?} visibility={:?} legal_effect={:?} credibility={:?} actionability={:?}",
        snapshot.artifact,
        snapshot.existence,
        snapshot.visibility,
        snapshot.legal_effect,
        snapshot.credibility,
        snapshot.actionability
    )
}

fn format_competition_discount_summary(discount: &CompetitionDiscount) -> String {
    format!(
        ", competition=domain={:?} competitors={:?} discount={} pre={} post={}",
        discount.domain,
        discount.observed_competitors,
        discount.effective_discount.value(),
        discount.pre_discount_motive,
        discount.post_discount_motive,
    )
}

fn format_source_reliability_discount_summary(discount: &SourceReliabilityDiscount) -> String {
    format!(
        ", source_reliability=entity={} commodity={:?} failure={} pre={} post={}",
        discount.source_entity,
        discount.commodity,
        discount.failure_ratio_permille,
        discount.pre_discount_motive,
        discount.post_discount_motive,
    )
}

fn format_source_composite_summary(rank: &SourceCompositeRank) -> String {
    format!(
        ", source_composite=entity={} commodity={:?} trust={} wait={} cap={} composite={}",
        rank.source_entity,
        rank.commodity,
        rank.trust_factor_permille,
        rank.wait_factor_permille,
        rank.capacity_factor_permille,
        rank.composite_permille,
    )
}

fn format_selected_plan(selected_plan: &SelectedPlanTrace) -> String {
    let step_kinds = selected_plan
        .steps
        .iter()
        .map(|step| format!("{:?}", step.op_kind))
        .collect::<Vec<_>>()
        .join("->");
    let next_step = selected_plan
        .next_step
        .as_ref()
        .map_or_else(|| "none".to_string(), |step| format!("{:?}", step.op_kind));
    let search = selected_plan.search_provenance.as_ref().map_or_else(
        || "none".to_string(),
        format_selected_plan_search_provenance,
    );
    format!(
        "{:?}[steps={}, next_index={:?}, next_step={next_step}, path={step_kinds}, primary={}, total={}, side_benefits={}, search={search}]",
        selected_plan.terminal_kind,
        selected_plan.steps.len(),
        selected_plan.next_step_index,
        selected_plan.primary_motive,
        selected_plan.total_value,
        selected_plan.side_benefits.len(),
    )
}

fn format_selected_plan_search_provenance(provenance: &SelectedPlanSearchProvenance) -> String {
    let pruning = provenance.root_travel_pruning.as_ref().map_or_else(
        || "none".to_string(),
        |trace| {
            let retained = trace
                .retained
                .iter()
                .map(|successor| {
                    format!(
                        "{:?}[base={}, threat={}, penalty={}, direct={}, remain={}, total={}]",
                        successor.destination,
                        successor.base_ticks,
                        successor.threat_permille.value(),
                        successor.penalty_ticks,
                        successor.direct_perceived_cost,
                        successor.remaining_travel_ticks,
                        successor.projected_total_cost
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let pruned = trace
                .pruned
                .iter()
                .map(|successor| {
                    format!(
                        "{:?}[base={}, threat={}, penalty={}, direct={}, remain={}, total={}]",
                        successor.destination,
                        successor.base_ticks,
                        successor.threat_permille.value(),
                        successor.penalty_ticks,
                        successor.direct_perceived_cost,
                        successor.remaining_travel_ticks,
                        successor.projected_total_cost
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "from={:?}@{}, kept=[{}], pruned=[{}]",
                trace.current_place, trace.current_remaining_travel_ticks, retained, pruned,
            )
        },
    );
    let selected = provenance.selected_root_travel_destination.map_or_else(
        || "none".to_string(),
        |destination| format!("{destination:?}"),
    );
    format!(
        "expansions={}, root_remaining={}, selected_root_travel={}, pruning={pruning}",
        provenance.expansions_used, provenance.root_remaining_travel_ticks, selected
    )
}

fn format_same_goal_planning_trace_summary(trace: &SameGoalPlanningTrace) -> String {
    let trigger = trace
        .continuation_trigger
        .map_or_else(|| "none".to_string(), format_opportunity_key);
    let stop = match trace.stop_reason {
        SameGoalPlanningStopReason::EncounteredDifferentGoal { next_goal } => {
            format!("EncounteredDifferentGoal({})", format_goal_key(&next_goal))
        }
        SameGoalPlanningStopReason::ReachedCandidatePlanCap => {
            "ReachedCandidatePlanCap".to_string()
        }
        SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities => {
            "ExhaustedAdmittedOpportunities".to_string()
        }
    };
    format!(", same_goal=trigger={trigger}, stop={stop}")
}

impl Default for DecisionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a single frame transition event for human-readable output.
fn format_frame_transition_kind(kind: &FrameTransitionKind) -> String {
    match kind {
        FrameTransitionKind::Created {
            goal,
            domain_tag,
            patience_limit,
            assumptions_count,
        } => format!(
            "FRAME_CREATED: goal={:?}, domain={domain_tag:?}, patience={patience_limit}, assumptions={assumptions_count}",
            goal.kind,
        ),
        FrameTransitionKind::Progressed { tick } => {
            format!("FRAME_PROGRESSED: tick={}", tick.0)
        }
        FrameTransitionKind::Suspended { reason, tick } => {
            format!("FRAME_SUSPENDED: reason={reason:?}, tick={}", tick.0)
        }
        FrameTransitionKind::Resumed { tick } => {
            format!("FRAME_RESUMED: tick={}", tick.0)
        }
        FrameTransitionKind::Exhausted {
            stalled_ticks,
            patience_limit,
            blocked_intent_recorded,
        } => format!(
            "FRAME_EXHAUSTED: stalled={stalled_ticks}/{patience_limit}, blocked={blocked_intent_recorded}"
        ),
        FrameTransitionKind::Cleared {
            reason,
            failed_assumption,
        } => failed_assumption.as_ref().map_or_else(
            || format!("FRAME_CLEARED: reason={reason:?}"),
            |assumption| {
                format!(
                    "FRAME_CLEARED: reason={reason:?}, failed={}",
                    format_frame_assumption(assumption)
                )
            },
        ),
    }
}

fn format_frame_assumption(assumption: &FrameAssumption) -> String {
    match assumption {
        FrameAssumption::TargetAlive(entity) => format!("TargetAlive(entity={entity:?})"),
        FrameAssumption::RouteExists { from, to } => {
            format!("RouteExists(from={from:?}, to={to:?})")
        }
        FrameAssumption::NoCriticalThreat => "NoCriticalThreat".to_string(),
        FrameAssumption::CommodityAvailableAt { commodity, place } => {
            format!("CommodityAvailableAt(commodity={commodity:?}, place={place:?})")
        }
        FrameAssumption::NeedSafeUntilTick { need, until_tick } => {
            format!("NeedSafeUntilTick {{ need: {need:?}, until_tick: {until_tick:?} }}")
        }
    }
}

/// Compact one-line summary of frame transitions for `summary()`.
fn format_frame_transition_summary(trace: Option<&FrameTransitionTrace>) -> String {
    let Some(trace) = trace else {
        return String::new();
    };
    let kinds: Vec<&str> = trace
        .transitions
        .iter()
        .map(|t| match t {
            FrameTransitionKind::Created { .. } => "created",
            FrameTransitionKind::Progressed { .. } => "progressed",
            FrameTransitionKind::Suspended { .. } => "suspended",
            FrameTransitionKind::Resumed { .. } => "resumed",
            FrameTransitionKind::Exhausted { .. } => "exhausted",
            FrameTransitionKind::Cleared { .. } => "cleared",
        })
        .collect();
    format!(", frame=[{}]", kinds.join(","))
}

// ── Knowledge Path Formatting ────────────────────────────────────

fn format_perception_source(source: &PerceptionSource) -> String {
    match source {
        PerceptionSource::DirectObservation => "DirectObservation".to_string(),
        PerceptionSource::Report { from, chain_len } => {
            format!("Report(from={from:?}, chain={chain_len})")
        }
        PerceptionSource::Rumor { chain_len } => format!("Rumor(chain={chain_len})"),
        PerceptionSource::Inference => "Inference".to_string(),
    }
}

fn format_institutional_knowledge_source(source: &InstitutionalKnowledgeSource) -> String {
    match source {
        InstitutionalKnowledgeSource::DirectObservation => "DirectObservation".to_string(),
        InstitutionalKnowledgeSource::WitnessedEvent => "WitnessedEvent".to_string(),
        InstitutionalKnowledgeSource::Report { from, chain_len } => {
            format!("Report(from={from:?}, chain={chain_len})")
        }
        InstitutionalKnowledgeSource::RecordConsultation { record, entry_id } => {
            format!(
                "RecordConsultation(record={record:?}, entry={})",
                entry_id.0
            )
        }
        InstitutionalKnowledgeSource::SelfDeclaration => "SelfDeclaration".to_string(),
    }
}

fn format_belief_aspect(aspect: &BeliefAspect) -> String {
    match aspect {
        BeliefAspect::LocationAt { place } => format!("at {place:?}"),
        BeliefAspect::HasCommodity { commodity } => format!("has {commodity:?}"),
        BeliefAspect::HasWorkstation { tag } => format!("has workstation {tag:?}"),
        BeliefAspect::IsResourceSource { commodity } => {
            format!("resource source for {commodity:?}")
        }
        BeliefAspect::Alive => "alive".to_string(),
        BeliefAspect::Dead => "dead".to_string(),
        BeliefAspect::Wounded => "wounded".to_string(),
        BeliefAspect::Hostile => "hostile".to_string(),
    }
}

fn format_self_knowledge(sk: &SelfKnowledgeProvenance) -> String {
    match sk {
        SelfKnowledgeProvenance::NeedLevel { need, permille } => {
            format!("NeedLevel({need:?}, {} permille)", permille.value())
        }
        SelfKnowledgeProvenance::OwnWounds { count } => format!("OwnWounds(count={count})"),
        SelfKnowledgeProvenance::OwnCommodity {
            commodity,
            quantity,
        } => format!("OwnCommodity({commodity:?}, qty={})", quantity.0),
        SelfKnowledgeProvenance::MerchantIdentity => "MerchantIdentity".to_string(),
    }
}

fn format_belief_provenance(bp: &BeliefProvenance) -> String {
    let aspect = format_belief_aspect(&bp.aspect);
    let source = format_perception_source(&bp.source);
    format!(
        "{:?} {aspect} — {source} @ tick {}",
        bp.subject, bp.observed_tick.0
    )
}

fn format_institutional_belief_provenance(ibp: &InstitutionalBeliefProvenance) -> String {
    let claim = format_institutional_claim(&ibp.claim);
    let source = format_institutional_knowledge_source(&ibp.source);
    let learned_at = ibp
        .learned_at
        .map_or_else(String::new, |place| format!(", learned_at={place:?}"));
    format!(
        "{claim} — {source} @ tick {}{learned_at}",
        ibp.learned_tick.0
    )
}

fn format_institutional_claim(claim: &InstitutionalClaim) -> String {
    match claim {
        InstitutionalClaim::OfficeHolder {
            office,
            holder,
            effective_tick,
        } => {
            let holder_str = holder.map_or_else(|| "vacant".to_string(), |h| format!("{h:?}"));
            format!(
                "OfficeHolder(office={office:?}, holder={holder_str}, tick={})",
                effective_tick.0
            )
        }
        InstitutionalClaim::FactionMembership {
            faction,
            member,
            active,
            effective_tick,
        } => format!(
            "FactionMembership(faction={faction:?}, member={member:?}, active={active}, tick={})",
            effective_tick.0
        ),
        InstitutionalClaim::FactionRallyPoint {
            faction,
            rally_place,
            effective_tick,
        } => {
            let rally_str = rally_place.map_or_else(|| "none".to_string(), |p| format!("{p:?}"));
            format!(
                "FactionRallyPoint(faction={faction:?}, rally_place={rally_str}, tick={})",
                effective_tick.0
            )
        }
        InstitutionalClaim::SupportDeclaration {
            office,
            supporter,
            candidate,
            effective_tick,
        } => {
            let cand_str = candidate.map_or_else(|| "none".to_string(), |c| format!("{c:?}"));
            format!(
                "SupportDeclaration(office={office:?}, supporter={supporter:?}, candidate={cand_str}, tick={})",
                effective_tick.0
            )
        }
        InstitutionalClaim::ForceControl {
            office,
            controller,
            contested,
            effective_tick,
        } => {
            let ctrl_str = controller.map_or_else(|| "none".to_string(), |c| format!("{c:?}"));
            format!(
                "ForceControl(office={office:?}, controller={ctrl_str}, contested={contested}, tick={})",
                effective_tick.0
            )
        }
        InstitutionalClaim::Accusation {
            accuser,
            accused,
            violation_id,
            effective_tick,
            ..
        } => format!(
            "Accusation(accuser={accuser:?}, accused={accused:?}, violation_id={violation_id:?}, tick={})",
            effective_tick.0
        ),
        InstitutionalClaim::Verdict {
            accused,
            violation_id,
            punishment,
            effective_tick,
        } => format!(
            "Verdict(accused={accused:?}, violation_id={violation_id:?}, punishment={punishment:?}, tick={})",
            effective_tick.0
        ),
        InstitutionalClaim::ArtifactCredibilityRefutation {
            artifact,
            evidence,
            effective_tick,
        } => format!(
            "ArtifactCredibilityRefutation(artifact={artifact:?}, evidence={evidence:?}, tick={})",
            effective_tick.0
        ),
        InstitutionalClaim::MissingPersonStatus {
            subject,
            reporter,
            status,
            effective_tick,
        } => format!(
            "MissingPersonStatus(subject={subject:?}, reporter={reporter:?}, status={status:?}, tick={})",
            effective_tick.0
        ),
    }
}

/// Render knowledge path lines for a candidate. Returns the lines to append
/// (including the "Knowledge path:" header). Empty if path is empty.
fn format_knowledge_path(kp: &KnowledgePath) -> Vec<String> {
    if kp.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["    Knowledge path:".to_string()];
    for sk in &kp.self_knowledge {
        lines.push(format!("      self: {}", format_self_knowledge(sk)));
    }
    for bp in &kp.entity_beliefs {
        lines.push(format!("      belief: {}", format_belief_provenance(bp)));
    }
    for ibp in &kp.institutional_beliefs {
        lines.push(format!(
            "      institutional: {}",
            format_institutional_belief_provenance(ibp)
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        AcquisitionQuantity, CommodityPurpose, ExplorationMotivation, GoalKind, HypothesisKind,
        OpportunityAnchor, Tick,
    };

    #[test]
    fn format_goal_kind_emits_acquire_quantity_fields() {
        let quantity = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(2).unwrap(),
            desired_target: std::num::NonZeroU16::new(5).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(123).unwrap(),
        };
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity,
        };

        let formatted = format_goal_kind(&goal);

        assert!(
            formatted.contains("AcquireCommodity(SelfConsume)"),
            "expected dispatch label in trace, got: {formatted}"
        );
        assert!(
            formatted.contains("desired_min"),
            "expected desired_min in trace, got: {formatted}"
        );
        assert!(
            formatted.contains("desired_target"),
            "expected desired_target in trace, got: {formatted}"
        );
        assert!(
            formatted.contains("horizon_ticks"),
            "expected horizon_ticks in trace, got: {formatted}"
        );
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn candidate_trace_default_has_empty_damped_vec() {
        let trace = CandidateTrace::default();

        assert!(trace.damped.is_empty());
    }

    #[test]
    fn candidate_damping_entry_renders_survey_memory_negative_with_full_provenance() {
        let place = entity(42);
        let hypothesis = HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        };
        let goal_key = GoalKey::new(GoalKind::ExploreLocation {
            target_place: place,
            motivating_need: ExplorationMotivation::NeedDriven(
                worldwake_core::HomeostaticNeedId::Hunger,
            ),
            hypothesis,
        });
        let entry = CandidateDampingEntry {
            goal_key,
            reason: CandidateDampingReason::SurveyMemoryNegative {
                place,
                hypothesis,
                recorded_tick: Tick(312),
                confidence: Permille::new(850).unwrap(),
            },
        };

        let rendered = format_candidate_damping_entry(&entry);

        assert!(rendered.contains("ExploreLocation"), "{rendered}");
        assert!(rendered.contains("damped by SurveyMemory"), "{rendered}");
        assert!(rendered.contains("found=false"), "{rendered}");
        assert!(rendered.contains("tick 312"), "{rendered}");
        assert!(rendered.contains("confidence=850"), "{rendered}");
        assert!(rendered.contains(&place.to_string()), "{rendered}");
        assert!(rendered.contains("MayContainCommodity"), "{rendered}");
        assert!(rendered.contains("Apple"), "{rendered}");
    }

    #[test]
    fn planning_summary_renders_ranked_artifact_axis_snapshot() {
        let artifact = entity(44);
        let goal = GoalKey::new(GoalKind::FulfillBounty { bounty: artifact });
        let opportunity = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Entity(artifact),
        };
        let trace = AgentDecisionTrace {
            agent: entity(1),
            tick: Tick(8),
            opportunity_compiler_load: None,
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: crate::DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: vec![opportunity],
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: vec![RankedGoalSummary {
                        opportunity,
                        priority_class: crate::GoalPriorityClass::Medium,
                        motive_score: 500,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: Some(ArtifactAxisSnapshot {
                            artifact,
                            existence: worldwake_core::ArtifactExistence::Exists,
                            visibility: worldwake_core::ArtifactVisibility::Posted {
                                place: entity(9),
                            },
                            legal_effect: worldwake_core::ArtifactLegalEffect::Active {
                                expires_at: Some(Tick(99)),
                            },
                            credibility: worldwake_core::ArtifactCredibility::Credible,
                            actionability: worldwake_core::ArtifactActionability::Actionable,
                        }),
                    }],
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    damped: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_violation_detection: Vec::new(),
                },
                planning: PlanSearchTrace {
                    attempts: Vec::new(),
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: Some(opportunity),
                    selected_plan: None,
                    selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: None,
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                frame_transition: None,
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        };

        let rendered = trace.outcome.summary();

        assert!(rendered.contains("artifact_axes=artifact="), "{rendered}");
        assert!(rendered.contains("existence=Exists"), "{rendered}");
        assert!(rendered.contains("visibility=Posted"), "{rendered}");
        assert!(rendered.contains("legal_effect=Active"), "{rendered}");
        assert!(rendered.contains("credibility=Credible"), "{rendered}");
        assert!(rendered.contains("actionability=Actionable"), "{rendered}");
    }

    fn dead_trace(agent: EntityId, tick: Tick) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent,
            tick,
            opportunity_compiler_load: None,
            outcome: DecisionOutcome::Dead,
        }
    }

    fn default_opportunity(goal: GoalKey) -> OpportunityKey {
        OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::None,
        }
    }

    fn sample_competition_discount() -> CompetitionDiscount {
        CompetitionDiscount {
            observed_competitors: vec![entity(8), entity(9)],
            domain: ActionDomain::Production,
            effective_discount: Permille::new(400).unwrap(),
            pre_discount_motive: 700,
            post_discount_motive: 420,
        }
    }

    fn sample_source_reliability_discount() -> SourceReliabilityDiscount {
        SourceReliabilityDiscount {
            source_entity: entity(12),
            commodity: CommodityKind::Bread,
            failure_ratio_permille: 500,
            pre_discount_motive: 700,
            post_discount_motive: 350,
        }
    }

    fn sample_source_composite_rank() -> SourceCompositeRank {
        SourceCompositeRank {
            source_entity: entity(12),
            commodity: CommodityKind::Bread,
            trust_factor_permille: 900,
            wait_factor_permille: 800,
            capacity_factor_permille: 1200,
            composite_permille: 864,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn goal_trace(
        tick: Tick,
        generated: Vec<GoalKey>,
        suppressed: Vec<GoalKey>,
        zero_motive: Vec<GoalKey>,
        ranked: Vec<RankedGoalSummary>,
        selected: Option<GoalKey>,
        selected_plan_source: Option<SelectedPlanSource>,
        plan_continued: bool,
        omitted_political: Vec<PoliticalCandidateOmission>,
        omitted_bandit: Vec<BanditCandidateOmission>,
        omitted_social: Vec<SocialCandidateOmission>,
    ) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent: entity(1),
            tick,
            opportunity_compiler_load: None,
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: crate::DirtySet::default(),
                plan_continued,
                candidates: CandidateTrace {
                    generated: generated.into_iter().map(default_opportunity).collect(),
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked,
                    top_ranked_comparison: None,
                    suppressed,
                    damped: Vec::new(),
                    zero_motive,
                    omitted_political,
                    omitted_bandit,
                    omitted_social,
                    omitted_violation_detection: vec![],
                },
                planning: PlanSearchTrace {
                    attempts: Vec::new(),
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: selected.map(default_opportunity),
                    selected_plan: None,
                    selected_plan_source,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: None,
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                frame_transition: None,
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        }
    }

    #[test]
    fn candidate_trace_retains_place_filter_counters() {
        let goal = GoalKey::new(GoalKind::Sleep);
        let trace = AgentDecisionTrace {
            agent: entity(1),
            tick: Tick(5),
            opportunity_compiler_load: None,
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: crate::DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: vec![default_opportunity(goal)],
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 12,
                    places_after_belief_filter: 3,
                    ranked: Vec::new(),
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    damped: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_violation_detection: Vec::new(),
                },
                planning: PlanSearchTrace {
                    attempts: Vec::new(),
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: None,
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: None,
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
                frame_transition: None,
            })),
        };

        let DecisionOutcome::Planning(planning) = trace.outcome else {
            panic!("expected planning trace");
        };
        assert_eq!(planning.candidates.places_reachable, 12);
        assert_eq!(planning.candidates.places_after_belief_filter, 3);
    }

    #[test]
    fn sink_record_and_query() {
        let mut sink = DecisionTraceSink::new();

        let agent_a = entity(0);
        let agent_b = entity(1);
        let tick_1 = Tick(1);
        let tick_2 = Tick(2);

        sink.record(dead_trace(agent_a, tick_1));
        sink.record(dead_trace(agent_a, tick_2));
        sink.record(dead_trace(agent_b, tick_1));

        // traces() returns all 3
        assert_eq!(sink.traces().len(), 3);

        // traces_for(agent_a) returns 2
        assert_eq!(sink.traces_for(agent_a).len(), 2);

        // traces_for(agent_b) returns 1
        assert_eq!(sink.traces_for(agent_b).len(), 1);

        // trace_at(agent_a, tick_1) returns the correct one
        let t = sink.trace_at(agent_a, tick_1).unwrap();
        assert_eq!(t.agent, agent_a);
        assert_eq!(t.tick, tick_1);
        assert!(matches!(t.outcome, DecisionOutcome::Dead));
    }

    #[test]
    fn sink_records_opportunity_compiler_load_by_agent_tick() {
        let mut sink = DecisionTraceSink::new();
        let agent = entity(0);
        let tick = Tick(4);
        let load = OpportunityCompilerLoad {
            compiled_count: 3,
            salience_floored: 1,
            learned_memory_damped: 2,
            cap_truncated: 1,
        };
        let mut trace = dead_trace(agent, tick);
        trace.opportunity_compiler_load = Some(load);

        sink.record(trace);

        assert_eq!(sink.opportunity_compiler_load(agent, tick), Some(&load));
    }

    #[test]
    fn sink_clear() {
        let mut sink = DecisionTraceSink::new();
        let agent = entity(0);

        sink.record(dead_trace(agent, Tick(1)));
        sink.record(dead_trace(agent, Tick(2)));
        assert_eq!(sink.traces().len(), 2);

        sink.clear();
        assert!(sink.traces().is_empty());
    }

    #[test]
    fn trace_at_missing() {
        let sink = DecisionTraceSink::new();
        let agent = entity(0);

        assert!(sink.trace_at(agent, Tick(99)).is_none());
    }

    #[test]
    fn format_frame_assumption_renders_need_safe_until_tick() {
        let assumption = FrameAssumption::NeedSafeUntilTick {
            need: worldwake_core::HomeostaticNeedId::Hunger,
            until_tick: Tick(412),
        };

        let rendered = format_frame_assumption(&assumption);

        assert_eq!(
            rendered,
            "NeedSafeUntilTick { need: Hunger, until_tick: Tick(412) }"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected() {
        let office = entity(10);
        let rival = entity(11);
        let omitted_goal = GoalKind::ClaimOffice { office };
        let suppressed_goal = GoalKind::Sleep;
        let zero_motive_goal = GoalKind::Wash;
        let outranked_goal = GoalKind::TreatWounds {
            patient: entity(12),
        };
        let selected_goal = GoalKind::ReduceDanger;
        let generated_only_goal = GoalKind::Relieve;
        let absent_goal = GoalKind::EngageHostile { target: entity(99) };

        let trace = goal_trace(
            Tick(5),
            vec![
                GoalKey::from(&suppressed_goal),
                GoalKey::from(&zero_motive_goal),
                GoalKey::from(&outranked_goal),
                GoalKey::from(&selected_goal),
                GoalKey::from(&generated_only_goal),
            ],
            vec![GoalKey::from(&suppressed_goal)],
            vec![GoalKey::from(&zero_motive_goal)],
            vec![
                RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::from(&selected_goal)),
                    priority_class: GoalPriorityClass::High,
                    motive_score: 900,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                },
                RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::from(&outranked_goal)),
                    priority_class: GoalPriorityClass::Medium,
                    motive_score: 600,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                },
            ],
            Some(GoalKey::from(&selected_goal)),
            Some(SelectedPlanSource::SearchSelection),
            false,
            vec![PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office,
                candidate: None,
                reason: PoliticalCandidateOmissionReason::ForceSuccessionLaw,
            }],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(
            trace.goal_status(&omitted_goal),
            GoalTraceStatus::OmittedPolitical(PoliticalCandidateOmissionReason::ForceSuccessionLaw)
        );
        assert_eq!(
            trace.goal_status(&suppressed_goal),
            GoalTraceStatus::Suppressed
        );
        assert_eq!(
            trace.goal_status(&zero_motive_goal),
            GoalTraceStatus::ZeroMotive
        );
        assert_eq!(
            trace.goal_status(&outranked_goal),
            GoalTraceStatus::Ranked {
                rank: 1,
                selected: false,
            }
        );
        assert_eq!(
            trace.goal_status(&selected_goal),
            GoalTraceStatus::Ranked {
                rank: 0,
                selected: true,
            }
        );
        assert_eq!(
            trace.goal_status(&generated_only_goal),
            GoalTraceStatus::GeneratedOnly
        );
        assert_eq!(
            trace.goal_status(&absent_goal),
            GoalTraceStatus::NotGenerated
        );

        let support_goal = GoalKind::SupportCandidateForOffice {
            office,
            candidate: rival,
        };
        let support_trace = goal_trace(
            Tick(6),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            vec![PoliticalCandidateOmission {
                family: PoliticalGoalFamily::SupportCandidateForOffice,
                office,
                candidate: Some(rival),
                reason: PoliticalCandidateOmissionReason::CandidateNotEligible,
            }],
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            support_trace.goal_status(&support_goal),
            GoalTraceStatus::OmittedPolitical(
                PoliticalCandidateOmissionReason::CandidateNotEligible
            )
        );

        let conflicted_goal = GoalKind::ClaimOffice { office: entity(44) };
        let conflicted_trace = goal_trace(
            Tick(7),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            vec![PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office: entity(44),
                candidate: None,
                reason: PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted,
            }],
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            conflicted_trace.goal_status(&conflicted_goal),
            GoalTraceStatus::OmittedPolitical(
                PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted
            )
        );
    }

    #[test]
    fn goal_status_reports_social_omission_reason() {
        let listener = entity(10);
        let subject = entity(11);
        let share_goal = GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };

        let trace = goal_trace(
            Tick(7),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            Vec::new(),
            Vec::new(),
            vec![SocialCandidateOmission {
                listener,
                topic: TellTopic::EntityBelief { subject },
                reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
            }],
        );

        assert_eq!(
            trace.goal_status(&share_goal),
            GoalTraceStatus::OmittedSocial(
                TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief
            )
        );
    }

    #[test]
    fn goal_status_reports_bandit_omission_reason() {
        let faction = entity(12);
        let regroup_goal = GoalKind::RegroupWithFaction { faction };

        let trace = goal_trace(
            Tick(7),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            Vec::new(),
            vec![BanditCandidateOmission {
                family: BanditGoalFamily::RegroupWithFaction,
                faction,
                reason: BanditCandidateOmissionReason::MissingRallyBelief,
            }],
            Vec::new(),
        );

        assert_eq!(
            trace.goal_status(&regroup_goal),
            GoalTraceStatus::OmittedBandit(BanditCandidateOmissionReason::MissingRallyBelief)
        );
    }

    #[test]
    fn goal_status_reports_social_direct_observability_omission_reason() {
        let listener = entity(10);
        let subject = entity(11);
        let share_goal = GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };

        let trace = goal_trace(
            Tick(7),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            Vec::new(),
            Vec::new(),
            vec![SocialCandidateOmission {
                listener,
                topic: TellTopic::EntityBelief { subject },
                reason: TellTopicOmissionReason::DirectlyObservableByListener,
            }],
        );

        assert_eq!(
            trace.goal_status(&share_goal),
            GoalTraceStatus::OmittedSocial(TellTopicOmissionReason::DirectlyObservableByListener)
        );
    }

    #[test]
    fn political_omission_helper_only_matches_political_goal_families() {
        let office = entity(21);
        let candidate = entity(22);
        let omissions = vec![
            PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office,
                candidate: None,
                reason: PoliticalCandidateOmissionReason::ForceSuccessionLaw,
            },
            PoliticalCandidateOmission {
                family: PoliticalGoalFamily::SupportCandidateForOffice,
                office,
                candidate: Some(candidate),
                reason: PoliticalCandidateOmissionReason::CandidateNotEligible,
            },
        ];

        assert_eq!(
            omitted_political_reason_for_goal(&omissions, &GoalKind::ClaimOffice { office }),
            Some(PoliticalCandidateOmissionReason::ForceSuccessionLaw)
        );
        assert_eq!(
            omitted_political_reason_for_goal(
                &omissions,
                &GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            Some(PoliticalCandidateOmissionReason::CandidateNotEligible)
        );
        assert_eq!(
            omitted_political_reason_for_goal(
                &omissions,
                &GoalKind::ShareBelief {
                    listener: entity(23),
                    topic: TellTopic::EntityBelief { subject: office },
                    communication_class: worldwake_core::CommunicationClass::Gossip,
                }
            ),
            None
        );
    }

    #[test]
    fn social_omission_helper_only_matches_share_belief_goals() {
        let listener = entity(30);
        let subject = entity(31);
        let omissions = vec![SocialCandidateOmission {
            listener,
            topic: TellTopic::EntityBelief { subject },
            reason: TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        }];

        assert_eq!(
            omitted_social_reason_for_goal(
                &omissions,
                &GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                    communication_class: worldwake_core::CommunicationClass::Gossip,
                }
            ),
            Some(TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief)
        );
        assert_eq!(
            omitted_social_reason_for_goal(&omissions, &GoalKind::ClaimOffice { office: subject }),
            None
        );
    }

    #[test]
    fn bandit_omission_helper_only_matches_bandit_goal_families() {
        let faction = entity(40);
        let omissions = vec![
            BanditCandidateOmission {
                family: BanditGoalFamily::RegroupWithFaction,
                faction,
                reason: BanditCandidateOmissionReason::MissingRallyBelief,
            },
            BanditCandidateOmission {
                family: BanditGoalFamily::EstablishBanditCamp,
                faction,
                reason: BanditCandidateOmissionReason::MissingLocalControlledEdibleSupplies,
            },
        ];

        assert_eq!(
            omitted_bandit_reason_for_goal(&omissions, &GoalKind::RegroupWithFaction { faction }),
            Some(BanditCandidateOmissionReason::MissingRallyBelief)
        );
        assert_eq!(
            omitted_bandit_reason_for_goal(&omissions, &GoalKind::EstablishBanditCamp { faction }),
            Some(BanditCandidateOmissionReason::MissingLocalControlledEdibleSupplies)
        );
        assert_eq!(
            omitted_bandit_reason_for_goal(&omissions, &GoalKind::ClaimOffice { office: faction }),
            None
        );
    }

    #[test]
    fn goal_history_helpers_are_deterministic_and_preserve_continuation_metadata() {
        let agent = entity(1);
        let goal = GoalKind::ClaimOffice { office: entity(20) };
        let mut sink = DecisionTraceSink::new();
        sink.record(goal_trace(
            Tick(1),
            vec![GoalKey::from(&goal)],
            Vec::new(),
            Vec::new(),
            vec![RankedGoalSummary {
                opportunity: default_opportunity(GoalKey::from(&goal)),
                priority_class: GoalPriorityClass::Medium,
                motive_score: 700,
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Uncertain,
                acquisition_quantity: None,
                artifact_axes: None,
            }],
            Some(GoalKey::from(&goal)),
            Some(SelectedPlanSource::SearchSelection),
            false,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
        sink.record(goal_trace(
            Tick(2),
            vec![GoalKey::from(&goal)],
            Vec::new(),
            Vec::new(),
            vec![RankedGoalSummary {
                opportunity: default_opportunity(GoalKey::from(&goal)),
                priority_class: GoalPriorityClass::Medium,
                motive_score: 700,
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Uncertain,
                acquisition_quantity: None,
                artifact_axes: None,
            }],
            Some(GoalKey::from(&goal)),
            Some(SelectedPlanSource::SnapshotContinuation),
            true,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));

        let first = sink.goal_history_for(agent, &goal);
        let second = sink.goal_history_for(agent, &goal);
        assert_eq!(first, second, "history helpers must be deterministic");
        assert_eq!(first.len(), 2);
        assert_eq!(
            first[0],
            GoalHistoryEntry {
                tick: Tick(1),
                status: GoalTraceStatus::Ranked {
                    rank: 0,
                    selected: true,
                },
                plan_continued: false,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
            }
        );
        assert_eq!(
            first[1],
            GoalHistoryEntry {
                tick: Tick(2),
                status: GoalTraceStatus::Ranked {
                    rank: 0,
                    selected: true,
                },
                plan_continued: true,
                selected_plan_source: Some(SelectedPlanSource::SnapshotContinuation),
            }
        );
        assert_eq!(
            sink.goal_status_at(agent, Tick(99), &goal),
            GoalTraceStatus::NoTrace
        );
    }

    #[test]
    fn selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings() {
        let goal = GoalKey::new(GoalKind::AcquireCommodity {
            commodity: worldwake_core::CommodityKind::Bread,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let orchard = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(entity(11)),
        };
        let market = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(entity(12)),
        };
        let planning = PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![orchard, market],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![
                    RankedGoalSummary {
                        opportunity: orchard,
                        priority_class: GoalPriorityClass::High,
                        motive_score: 800,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Uncertain,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    },
                    RankedGoalSummary {
                        opportunity: market,
                        priority_class: GoalPriorityClass::High,
                        motive_score: 790,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    },
                ],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(market),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        };

        assert!(planning.candidates.generated_contains_goal(goal));
        assert!(planning.candidates.generated_contains_opportunity(orchard));
        assert!(planning.selection.selected_goal_is(goal));
        assert!(planning.selection.selected_opportunity_is(market));

        let selected = planning
            .selected_ranked_summary()
            .expect("selected opportunity should resolve to a ranked summary");
        assert_eq!(selected.opportunity, market);
        assert_eq!(selected.feasibility, FeasibilityHint::Likely);
        assert_eq!(
            planning
                .candidates
                .ranked_summaries_for_goal(goal)
                .into_iter()
                .map(|summary| summary.opportunity)
                .collect::<Vec<_>>(),
            vec![orchard, market]
        );
    }

    #[test]
    fn planning_pipeline_trace_portfolio_defaults_to_none_in_literal() {
        let planning = PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        };

        assert_eq!(planning.portfolio, None);
    }

    #[test]
    fn portfolio_trace_preserves_slot_contents() {
        let sleep = GoalKey::new(GoalKind::Sleep);
        let bread = GoalKey::new(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: worldwake_core::CommodityPurpose::Restock,
            quantity: AcquisitionQuantity::single(),
        });
        let mut slots = std::collections::BTreeMap::new();
        slots.insert(
            SlotKind::Survival,
            PortfolioSlotTrace {
                goal_key: sleep,
                motive_score: 900,
                feasibility: FeasibilityVerdict::Plausible,
            },
        );
        slots.insert(
            SlotKind::Economic,
            PortfolioSlotTrace {
                goal_key: bread,
                motive_score: 450,
                feasibility: FeasibilityVerdict::RejectedBeforeSearch {
                    reason: worldwake_core::Discrepancy::MissingObservation,
                },
            },
        );
        let trace = PortfolioTrace {
            slots,
            slots_attempted: 1,
        };

        assert_eq!(
            trace.slots.keys().copied().collect::<Vec<_>>(),
            vec![SlotKind::Survival, SlotKind::Economic]
        );
        assert_eq!(
            trace.slots.get(&SlotKind::Survival),
            Some(&PortfolioSlotTrace {
                goal_key: sleep,
                motive_score: 900,
                feasibility: FeasibilityVerdict::Plausible,
            })
        );
        assert_eq!(
            trace.slots.get(&SlotKind::Economic),
            Some(&PortfolioSlotTrace {
                goal_key: bread,
                motive_score: 450,
                feasibility: FeasibilityVerdict::RejectedBeforeSearch {
                    reason: worldwake_core::Discrepancy::MissingObservation,
                },
            })
        );
        assert_eq!(trace.slots_attempted, 1);
    }

    #[test]
    fn selected_goal_helper_derives_from_selected_opportunity() {
        let goal = GoalKey::new(GoalKind::ClaimOffice { office: entity(14) });
        let selection = SelectionTrace {
            selected_opportunity: Some(OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Entity(entity(15)),
            }),
            selected_plan: None,
            selected_plan_source: Some(SelectedPlanSource::SearchSelection),
            goal_switch: None,
            previous_goal: None,
            plan_replacement: None,
            snapshot_continuation: None,
        };

        assert_eq!(selection.selected_goal(), Some(goal));
        assert!(selection.selected_goal_is(goal));
        assert!(
            !SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            }
            .selected_goal_is(goal)
        );
    }

    #[test]
    fn candidate_trace_helpers_lookup_evidence_by_goal_and_opportunity() {
        let goal = GoalKey::new(GoalKind::ClaimOffice { office: entity(30) });
        let opportunity = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Entity(entity(31)),
        };
        let evidence = CandidateEvidenceTrace {
            opportunity,
            contributors: vec![CandidateEvidenceContributor {
                kind: CandidateEvidenceKind::OfficeParticipant,
                place: entity(32),
                entity: entity(31),
            }],
            exclusions: vec![],
            knowledge_path: KnowledgePath::default(),
            legality: None,
            pursuit: None,
            artifact_axes: None,
        };
        let candidates = CandidateTrace {
            generated: vec![opportunity],
            evidence: vec![evidence.clone()],
            fully_blocked_desires: vec![],
            places_reachable: 0,
            places_after_belief_filter: 0,
            ranked: vec![],
            top_ranked_comparison: None,
            suppressed: vec![],
            damped: Vec::new(),
            zero_motive: vec![],
            omitted_political: vec![],
            omitted_bandit: vec![],
            omitted_social: vec![],
            omitted_violation_detection: vec![],
        };

        assert_eq!(
            candidates.evidence_for_opportunity(opportunity),
            Some(&evidence)
        );
        assert_eq!(candidates.evidence_for_goal(goal), vec![&evidence]);
    }

    #[test]
    fn summary_dead_returns_non_empty_string() {
        let summary = DecisionOutcome::Dead.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("DEAD"));
    }

    #[test]
    fn summary_active_action_includes_action_name() {
        let outcome = DecisionOutcome::ActiveAction {
            action_def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            interrupt: InterruptTrace {
                decision: InterruptDecision::NoInterrupt,
                top_challenger: None,
                top_challenger_comparison: None,
            },
            frame_transition: None,
        };
        let summary = outcome.summary();
        assert!(summary.contains("ACTIVE"));
        assert!(summary.contains("eat"));
        assert!(summary.contains("NoInterrupt"));
    }

    #[test]
    fn summary_planning_includes_candidate_count() {
        use worldwake_core::GoalKind;
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::Sleep)),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 800,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(GoalKind::Sleep))),
                selected_plan: Some(SelectedPlanTrace {
                    steps: vec![PlannedStepSummary {
                        action_def_id: ActionDefId(1),
                        action_name: "sleep".to_string(),
                        op_kind: PlannerOpKind::Sleep,
                        targets: vec![],
                        estimated_ticks: 2,
                        binding_strictness: None,
                    }],
                    terminal_kind: PlanTerminalKind::GoalSatisfied,
                    next_step_index: Some(0),
                    next_step: Some(PlannedStepSummary {
                        action_def_id: ActionDefId(1),
                        action_name: "sleep".to_string(),
                        op_kind: PlannerOpKind::Sleep,
                        targets: vec![],
                        estimated_ticks: 2,
                        binding_strictness: None,
                    }),
                    search_provenance: Some(SelectedPlanSearchProvenance {
                        expansions_used: 3,
                        root_remaining_travel_ticks: 7,
                        selected_root_travel_destination: Some(entity(12)),
                        root_travel_pruning: Some(TravelPruningTrace {
                            current_place: entity(11),
                            current_remaining_travel_ticks: 7,
                            retained: vec![TravelSuccessorTrace {
                                destination: entity(12),
                                base_ticks: 2,
                                threat_permille: Permille::new(0).unwrap(),
                                penalty_ticks: 0,
                                direct_perceived_cost: 2,
                                remaining_travel_ticks: 5,
                                projected_total_cost: 7,
                            }],
                            pruned: vec![TravelSuccessorTrace {
                                destination: entity(13),
                                base_ticks: 4,
                                threat_permille: Permille::new(0).unwrap(),
                                penalty_ticks: 0,
                                direct_perceived_cost: 4,
                                remaining_travel_ticks: 9,
                                projected_total_cost: 13,
                            }],
                        }),
                    }),
                    primary_motive: 600,
                    total_value: 660,
                    side_benefits: vec![SideBenefitTrace {
                        goal_key: GoalKey::new(GoalKind::SellCommodity {
                            commodity: CommodityKind::Apple,
                        }),
                        at_place: entity(12),
                        estimated_value: 60,
                    }],
                }),
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));
        let summary = outcome.summary();
        assert!(summary.contains("PLAN"));
        assert!(summary.contains("candidates=1"));
        assert!(summary.contains("plans_found=0"));
        assert!(summary.contains("Sleep"));
        assert!(summary.contains("SearchSelection"));
        assert!(summary.contains("GoalSatisfied"));
        assert!(summary.contains("Sleep]") || summary.contains("path=Sleep"));
        assert!(summary.contains("primary=600"));
        assert!(summary.contains("total=660"));
        assert!(summary.contains("side_benefits=1"));
        assert!(summary.contains("expansions=3"));
        assert!(summary.contains("root_remaining=7"));
        assert!(summary.contains("selected_root_travel=EntityId"));
        assert!(summary.contains("pruned=["));
    }

    #[test]
    fn summary_planning_includes_same_goal_stop_and_replacement_kind() {
        let goal = GoalKey::new(GoalKind::RestockCommodity {
            commodity: worldwake_core::CommodityKind::Bread,
        });
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(goal)],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: Some(SameGoalPlanningTrace {
                    continuation_trigger: Some(OpportunityKey {
                        goal_key: goal,
                        anchor: OpportunityAnchor::Place(entity(22)),
                    }),
                    stop_reason: SameGoalPlanningStopReason::EncounteredDifferentGoal {
                        next_goal: GoalKey::new(GoalKind::Sleep),
                    },
                }),
            },
            selection: SelectionTrace {
                selected_opportunity: Some(OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Place(entity(23)),
                }),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: Some(goal),
                plan_replacement: Some(SelectedPlanReplacementTrace {
                    previous_goal: goal,
                    new_goal: goal,
                    previous_next_step: None,
                    new_next_step: None,
                    kind: SelectedPlanReplacementKind::SameGoalSiblingReplaced,
                }),
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("same_goal=trigger=RestockCommodity"));
        assert!(summary.contains("EncounteredDifferentGoal(Sleep)"));
        assert!(summary.contains("replacement=SameGoalSiblingReplaced"));
    }

    #[test]
    fn summary_planning_uses_trace_label_and_preserves_selected_goal_payload() {
        use worldwake_core::{GoalKind, OpportunityAnchor};

        let target = entity(41);
        let goal = GoalKey::new(GoalKind::EngageHostile { target });
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Entity(target),
                }],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: OpportunityKey {
                        goal_key: goal,
                        anchor: OpportunityAnchor::Entity(target),
                    },
                    priority_class: GoalPriorityClass::High,
                    motive_score: 700,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Likely,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Entity(target),
                }),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();
        assert!(summary.contains("selected=EngageHostile"));
        assert!(summary.contains("selected_opportunity=EngageHostile"));
        assert!(summary.contains("target"));
        assert!(summary.contains("slot: 41"));
    }

    #[test]
    fn summary_planning_includes_selected_competition_discount() {
        use worldwake_core::GoalKind;

        let discount = sample_competition_discount();
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::Sleep)),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: discount.post_discount_motive,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: Some(discount),
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(GoalKind::Sleep))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("competition=domain=Production"));
        assert!(summary.contains("discount=400"));
        assert!(summary.contains("pre=700"));
        assert!(summary.contains("post=420"));
    }

    #[test]
    fn summary_planning_includes_selected_source_reliability_discount() {
        use worldwake_core::GoalKind;

        let discount = sample_source_reliability_discount();
        let composite = sample_source_composite_rank();
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::Sleep)),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: discount.post_discount_motive,
                    provenance: None,
                    source_reliability_discount: Some(discount),
                    competition_discount: None,
                    source_composite: Some(composite),
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(GoalKind::Sleep))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("source_reliability=entity="));
        assert!(summary.contains("commodity=Bread"));
        assert!(summary.contains("failure=500"));
        assert!(summary.contains("pre=700"));
        assert!(summary.contains("post=350"));
        assert!(!summary.contains("wait_avg="));
        assert!(!summary.contains("wait_pen="));
        assert!(!summary.contains("cap_age="));
        assert!(!summary.contains("cap_sig="));
        assert!(summary.contains("source_composite=entity="));
        assert!(summary.contains("trust=900"));
        assert!(summary.contains("wait=800"));
        assert!(summary.contains("cap=1200"));
        assert!(summary.contains("composite=864"));
    }

    #[test]
    fn format_source_composite_summary_emits_factor_substrings() {
        let summary = format_source_composite_summary(&sample_source_composite_rank());

        assert!(summary.contains("source_composite=entity="));
        assert!(summary.contains("commodity=Bread"));
        assert!(summary.contains("trust=900"));
        assert!(summary.contains("wait=800"));
        assert!(summary.contains("cap=1200"));
        assert!(summary.contains("composite=864"));
    }

    #[test]
    fn summary_planning_omits_competition_discount_when_absent() {
        use worldwake_core::GoalKind;

        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::Sleep)),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 800,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(GoalKind::Sleep))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(!summary.contains("competition="));
    }

    #[test]
    fn summary_planning_includes_attempt_anchor() {
        use worldwake_core::{GoalKind, OpportunityAnchor};

        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![PlanAttemptTrace {
                    goal: GoalKey::new(GoalKind::Sleep),
                    opportunity_anchor: OpportunityAnchor::Place(entity(42)),
                    outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 2 },
                    target_belief_presence: TargetBeliefPresence::NotApplicable,
                    strategic_plan: None,
                    tactical_goal: None,
                    landmarks_extracted: 0,
                    landmark_orderings: 0,
                    binding_rejections: vec![],
                    expansion_summaries: vec![],
                }],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = format_outcome(&outcome, &ActionDefRegistry::new());
        assert!(summary.contains("anchor=Place("));
    }

    #[test]
    fn summary_planning_includes_desire_fully_blocked() {
        use worldwake_core::{CommodityKind, CommodityPurpose, GoalKind, OpportunityAnchor};

        let goal = GoalKey::new(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(goal)],
                evidence: vec![],
                fully_blocked_desires: vec![DesireFullyBlocked {
                    goal_key: goal,
                    blocked_opportunities: vec![
                        OpportunityKey {
                            goal_key: goal,
                            anchor: OpportunityAnchor::Place(entity(11)),
                        },
                        OpportunityKey {
                            goal_key: goal,
                            anchor: OpportunityAnchor::Place(entity(12)),
                        },
                    ],
                    blocker_matches: vec![],
                }],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = format_outcome(&outcome, &ActionDefRegistry::new());
        assert!(summary.contains("fully blocked desire"));
        assert!(summary.contains("AcquireCommodity(SelfConsume)"));
        assert!(summary.contains("commodity: Bread"));
        assert!(summary.contains("Place("));
    }

    #[test]
    fn summary_planning_includes_ranking_comparison() {
        use crate::ranking::{RankedGoalComparison, RankedGoalComparisonDimension};
        use worldwake_core::GoalKind;

        let winner = GoalKey::new(GoalKind::Sleep);
        let loser = GoalKey::new(GoalKind::Wash);
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(winner), default_opportunity(loser)],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![
                    RankedGoalSummary {
                        opportunity: default_opportunity(winner),
                        priority_class: GoalPriorityClass::Critical,
                        motive_score: 800,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    },
                    RankedGoalSummary {
                        opportunity: default_opportunity(loser),
                        priority_class: GoalPriorityClass::Critical,
                        motive_score: 600,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    },
                ],
                top_ranked_comparison: Some(RankedGoalComparison {
                    winner: default_opportunity(winner),
                    loser: default_opportunity(loser),
                    decisive_dimension: RankedGoalComparisonDimension::MotiveScore,
                }),
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(winner)),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("ranking=MotiveScore"));
        assert!(summary.contains("Sleep"));
        assert!(summary.contains("Wash"));
    }

    #[test]
    fn summary_planning_includes_selected_danger_provenance() {
        use worldwake_core::GoalKind;

        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(GoalKey::new(GoalKind::ReduceDanger))],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::ReduceDanger)),
                    priority_class: GoalPriorityClass::High,
                    motive_score: 700,
                    provenance: Some(RankedGoalProvenance::Danger(crate::DangerAssessment {
                        pressure: worldwake_core::Permille::new(600).unwrap(),
                        thresholds_present: true,
                        current_attackers: vec![entity(8)],
                        visible_hostiles: vec![entity(8), entity(9)],
                        hostile_targets: vec![entity(8), entity(9)],
                        has_wounds: true,
                        is_incapacitated: false,
                    })),
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(
                    GoalKind::ReduceDanger,
                ))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("danger=pressure=600"));
        assert!(summary.contains("attackers=["));
        assert!(summary.contains("visible_hostiles=["));
        assert!(summary.contains("hostile_targets=["));
    }

    #[test]
    fn summary_planning_includes_selected_drive_provenance() {
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(GoalKey::new(
                    GoalKind::ConsumeOwnedCommodity {
                        commodity: worldwake_core::CommodityKind::Bread,
                    },
                ))],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(
                        GoalKind::ConsumeOwnedCommodity {
                            commodity: worldwake_core::CommodityKind::Bread,
                        },
                    )),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 380_000,
                    provenance: Some(RankedGoalProvenance::Drive(
                        crate::RankedDriveGoalProvenance {
                            base_priority_class: GoalPriorityClass::High,
                            final_priority_class: GoalPriorityClass::Critical,
                            adjustment: Some(
                                crate::RankedPriorityAdjustment::ClottedWoundRecoveryPromotion,
                            ),
                            commodity_preference_rank: None,
                            motive_inputs: vec![crate::RankedDriveMotiveInput {
                                drive: crate::RankedDriveKind::Hunger,
                                pressure: worldwake_core::Permille::new(760).unwrap(),
                                weight: worldwake_core::Permille::new(500).unwrap(),
                                score: 380_000,
                                escalation_multiplier: worldwake_core::MultiplierPermille::IDENTITY,
                                relief_per_unit: worldwake_core::Permille::new(1000).unwrap(),
                                recovery_relevant: true,
                            }],
                        },
                    )),
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(
                    GoalKind::ConsumeOwnedCommodity {
                        commodity: worldwake_core::CommodityKind::Bread,
                    },
                ))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let summary = outcome.summary();

        assert!(summary.contains("drive=base=High final=Critical"));
        assert!(summary.contains("ClottedWoundRecoveryPromotion"));
        assert!(summary.contains("Hunger(pressure=760, weight=500, score=380000"));
    }

    #[test]
    fn summary_planning_includes_root_candidate_omissions_and_dependency_diagnostics() {
        use crate::planner_duration_contract::PlannerDurationDependency;

        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![default_opportunity(GoalKey::new(GoalKind::ClaimOffice {
                    office: entity(4),
                }))],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![RankedGoalSummary {
                    opportunity: default_opportunity(GoalKey::new(GoalKind::ClaimOffice {
                        office: entity(4),
                    })),
                    priority_class: GoalPriorityClass::High,
                    motive_score: 400,
                    provenance: None,
                    source_reliability_discount: None,
                    competition_discount: None,
                    source_composite: None,
                    feasibility: FeasibilityHint::Uncertain,
                    acquisition_quantity: None,
                    artifact_axes: None,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![PlanAttemptTrace {
                    goal: GoalKey::new(GoalKind::ClaimOffice { office: entity(4) }),
                    opportunity_anchor: OpportunityAnchor::None,
                    outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 1 },
                    strategic_plan: Some(vec![StrategicStepTrace {
                        destination: entity(8),
                        sub_goal: "AcquirePrerequisite(Firewood)".to_string(),
                        estimated_travel_ticks: 3,
                    }]),
                    tactical_goal: Some("TravelToGoal { destination: EntityId(8) }".to_string()),
                    landmarks_extracted: 2,
                    landmark_orderings: 1,
                    target_belief_presence: TargetBeliefPresence::Present,
                    binding_rejections: vec![],
                    expansion_summaries: vec![SearchExpansionSummary {
                        depth: 0,
                        remaining_travel_ticks: 0,
                        combined_places_count: 0,
                        prerequisite_places_count: 0,
                        candidates_generated: 1,
                        candidates_skipped: 1,
                        terminal_successors: 0,
                        non_terminal_before_beam: 0,
                        non_terminal_after_beam: 0,
                        found_goal_satisfied: false,
                        preferred_candidates: 1,
                        landmark_heuristic: 2,
                        ff_heuristic: Some(5),
                        helpful_action_count: 2,
                        travel_pruning: None,
                        prerequisite_guidance: None,
                        expansion_candidates: vec![],
                        root_candidates: vec![RootCandidateTrace {
                            def_id: ActionDefId(9),
                            action_name: "trade".to_string(),
                            op_kind: Some(PlannerOpKind::Trade),
                            authoritative_targets: vec![entity(7)],
                            planner_only: false,
                            payload_status: RootCandidatePayloadStatus::GoalSynthesized,
                            outcome: RootCandidateOutcome::Skipped(
                                RootCandidateSkipReason::DurationEstimateFailed {
                                    dependency: PlannerDurationDependency::ActorTradeDisposition,
                                },
                            ),
                            omitted_anchor: None,
                            source: CandidateSource::Emitter,
                        }],
                        root_omissions: vec![RootOperatorOmissionTrace {
                            op_kind: PlannerOpKind::PressForceClaim,
                            reason: RootOperatorOmissionReason::NoMatchingActionDef,
                            detail: None,
                        }],
                    }],
                }],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(default_opportunity(GoalKey::new(
                    GoalKind::ClaimOffice { office: entity(4) },
                ))),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));

        let mut action_defs = ActionDefRegistry::new();
        for id in 0..=9 {
            action_defs.register(worldwake_sim::ActionDef {
                id: ActionDefId(id),
                name: if id == 9 {
                    "trade".to_string()
                } else {
                    format!("action-{id}")
                },
                domain: worldwake_core::ActionDomain::Generic,
                actor_constraints: vec![],
                targets: vec![],
                preconditions: vec![],
                reservation_requirements: vec![],
                duration: worldwake_sim::DurationExpr::Fixed(std::num::NonZeroU32::new(1).unwrap()),
                body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
                attention_cost: worldwake_core::Permille::ZERO,
                interruptibility: worldwake_sim::Interruptibility::FreelyInterruptible,
                commit_conditions: vec![],
                visibility: worldwake_core::VisibilitySpec::SamePlace,
                causal_event_tags: std::collections::BTreeSet::new(),
                payload: worldwake_sim::ActionPayload::None,
                handler: worldwake_sim::ActionHandlerId(0),
                binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
                guard_template: None,
                expectation_template: vec![],
                effect_schema: worldwake_sim::EffectSchema::empty(),
            });
        }

        let summary = format_outcome(&outcome, &action_defs);
        assert!(summary.contains("root omission: PressForceClaim -> NoMatchingActionDef"));
        assert!(summary.contains("strategic plan: 1 steps, 2 landmarks, 1 orderings"));
        assert!(summary.contains("strategic step: dest="));
        assert!(
            summary.contains(
                "root candidate: trade op=Trade payload=GoalSynthesized outcome=Skipped(DurationEstimateFailed { dependency: ActorTradeDisposition })"
            )
        );
        assert!(summary.contains("1 preferred"));
        assert!(summary.contains("h_landmark=2"));
        assert!(summary.contains("h_ff=5, helpful_actions=2"));
    }

    #[test]
    fn binding_rejection_struct_holds_data() {
        let rej = BindingRejection {
            def_id: ActionDefId(42),
            rejected_targets: vec![entity(10), entity(11)],
            required_target: Some(entity(5)),
        };
        assert_eq!(rej.def_id, ActionDefId(42));
        assert_eq!(rej.rejected_targets.len(), 2);
        assert_eq!(rej.rejected_targets[0], entity(10));
        assert_eq!(rej.rejected_targets[1], entity(11));
        assert_eq!(rej.required_target, Some(entity(5)));
    }

    #[test]
    fn binding_rejection_with_no_required_target() {
        let rej = BindingRejection {
            def_id: ActionDefId(7),
            rejected_targets: vec![entity(3)],
            required_target: None,
        };
        assert_eq!(rej.required_target, None);
    }

    #[test]
    fn discrepancy_trace_struct_carries_typed_discrepancy() {
        let trace = DiscrepancyTrace {
            discrepancy: worldwake_core::Discrepancy::BeliefContradicted,
            blocker_key: BlockerKey {
                goal_key: GoalKey::new(GoalKind::Sleep),
                place: Some(entity(4)),
                target: Some(entity(5)),
                action_def: Some(ActionDefId(6)),
            },
            expires_tick: Tick(12),
        };

        assert_eq!(
            trace.discrepancy,
            worldwake_core::Discrepancy::BeliefContradicted
        );
        assert_eq!(trace.blocker_key.goal_key, GoalKey::new(GoalKind::Sleep));
        assert_eq!(trace.blocker_key.place, Some(entity(4)));
        assert_eq!(trace.blocker_key.target, Some(entity(5)));
        assert_eq!(trace.blocker_key.action_def, Some(ActionDefId(6)));
        assert_eq!(trace.expires_tick, Tick(12));
    }

    #[test]
    fn plan_attempt_trace_includes_binding_rejections() {
        use worldwake_core::GoalKind;
        let rejections = vec![
            BindingRejection {
                def_id: ActionDefId(1),
                rejected_targets: vec![entity(20)],
                required_target: Some(entity(10)),
            },
            BindingRejection {
                def_id: ActionDefId(2),
                rejected_targets: vec![entity(30)],
                required_target: Some(entity(10)),
            },
        ];
        let trace = PlanAttemptTrace {
            goal: GoalKey::new(GoalKind::Sleep),
            opportunity_anchor: OpportunityAnchor::Place(entity(9)),
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 5 },
            strategic_plan: None,
            tactical_goal: None,
            landmarks_extracted: 0,
            landmark_orderings: 0,
            target_belief_presence: TargetBeliefPresence::NotApplicable,
            binding_rejections: rejections,
            expansion_summaries: vec![],
        };
        assert_eq!(trace.binding_rejections.len(), 2);
        assert_eq!(trace.binding_rejections[0].def_id, ActionDefId(1));
        assert_eq!(trace.binding_rejections[1].rejected_targets[0], entity(30));
    }

    #[test]
    fn expansion_summary_default_and_debug_format() {
        let summary = SearchExpansionSummary {
            depth: 0,
            remaining_travel_ticks: 4,
            combined_places_count: 3,
            prerequisite_places_count: 1,
            candidates_generated: 12,
            candidates_skipped: 1,
            terminal_successors: 2,
            non_terminal_before_beam: 11,
            non_terminal_after_beam: 8,
            found_goal_satisfied: false,
            preferred_candidates: 3,
            landmark_heuristic: 2,
            ff_heuristic: Some(6),
            helpful_action_count: 4,
            travel_pruning: Some(TravelPruningTrace {
                current_place: entity(1),
                current_remaining_travel_ticks: 4,
                retained: vec![TravelSuccessorTrace {
                    destination: entity(2),
                    base_ticks: 2,
                    threat_permille: Permille::new(0).unwrap(),
                    penalty_ticks: 0,
                    direct_perceived_cost: 2,
                    remaining_travel_ticks: 2,
                    projected_total_cost: 4,
                }],
                pruned: vec![TravelSuccessorTrace {
                    destination: entity(3),
                    base_ticks: 3,
                    threat_permille: Permille::new(0).unwrap(),
                    penalty_ticks: 0,
                    direct_perceived_cost: 3,
                    remaining_travel_ticks: 6,
                    projected_total_cost: 9,
                }],
            }),
            prerequisite_guidance: None,
            expansion_candidates: vec![],
            root_candidates: vec![],
            root_omissions: vec![],
        };
        assert_eq!(summary.depth, 0);
        assert_eq!(summary.remaining_travel_ticks, 4);
        assert_eq!(summary.combined_places_count, 3);
        assert_eq!(summary.prerequisite_places_count, 1);
        assert_eq!(summary.candidates_generated, 12);
        assert_eq!(summary.candidates_skipped, 1);
        assert_eq!(summary.terminal_successors, 2);
        assert_eq!(summary.non_terminal_before_beam, 11);
        assert_eq!(summary.non_terminal_after_beam, 8);
        assert_eq!(summary.preferred_candidates, 3);
        assert_eq!(summary.landmark_heuristic, 2);
        assert_eq!(summary.ff_heuristic, Some(6));
        assert_eq!(summary.helpful_action_count, 4);
        assert!(!summary.found_goal_satisfied);
        assert!(summary.travel_pruning.is_some());

        // Verify Debug is derived and non-empty.
        let debug = format!("{summary:?}");
        assert!(debug.contains("SearchExpansionSummary"));
        assert!(debug.contains("depth: 0"));
        assert!(debug.contains("ff_heuristic: Some(6)"));
    }

    // ── Frame Transition Trace Tests ────────────────────────────────

    #[test]
    fn frame_transition_trace_created_format() {
        let kind = FrameTransitionKind::Created {
            goal: GoalKey::new(GoalKind::Sleep),
            domain_tag: IntentionDomainTag::Travel,
            patience_limit: 30,
            assumptions_count: 2,
        };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_CREATED"));
        assert!(formatted.contains("Travel"));
        assert!(formatted.contains("patience=30"));
        assert!(formatted.contains("assumptions=2"));
    }

    #[test]
    fn frame_transition_trace_progressed_format() {
        let kind = FrameTransitionKind::Progressed { tick: Tick(5) };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_PROGRESSED"));
        assert!(formatted.contains("tick=5"));
    }

    #[test]
    fn frame_transition_trace_suspended_format() {
        let kind = FrameTransitionKind::Suspended {
            reason: SuspensionReason::RouteBlocked,
            tick: Tick(7),
        };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_SUSPENDED"));
        assert!(formatted.contains("RouteBlocked"));
        assert!(formatted.contains("tick=7"));
    }

    #[test]
    fn frame_transition_trace_resumed_format() {
        let kind = FrameTransitionKind::Resumed { tick: Tick(10) };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_RESUMED"));
        assert!(formatted.contains("tick=10"));
    }

    #[test]
    fn frame_transition_trace_exhausted_format() {
        let kind = FrameTransitionKind::Exhausted {
            stalled_ticks: 30,
            patience_limit: 30,
            blocked_intent_recorded: true,
        };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_EXHAUSTED"));
        assert!(formatted.contains("stalled=30/30"));
        assert!(formatted.contains("blocked=true"));
    }

    #[test]
    fn frame_transition_trace_cleared_format() {
        let kind = FrameTransitionKind::Cleared {
            reason: FrameClearReason::PatienceExhausted,
            failed_assumption: None,
        };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_CLEARED"));
        assert!(formatted.contains("PatienceExhausted"));
    }

    #[test]
    fn format_frame_transition_cleared_with_failed_commodity_assumption_includes_payload() {
        let place = entity(7);
        let kind = FrameTransitionKind::Cleared {
            reason: FrameClearReason::AssumptionFailed,
            failed_assumption: Some(FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place,
            }),
        };

        let formatted = format_frame_transition_kind(&kind);

        assert!(formatted.contains("FRAME_CLEARED"));
        assert!(formatted.contains("AssumptionFailed"));
        assert!(formatted.contains("CommodityAvailableAt"));
        assert!(formatted.contains("Apple"));
        assert!(formatted.contains(&format!("{place:?}")));
    }

    #[test]
    fn frame_transition_summary_with_transitions() {
        let trace = FrameTransitionTrace {
            transitions: vec![
                FrameTransitionKind::Created {
                    goal: GoalKey::new(GoalKind::Sleep),
                    domain_tag: IntentionDomainTag::Generic,
                    patience_limit: 20,
                    assumptions_count: 1,
                },
                FrameTransitionKind::Progressed { tick: Tick(3) },
            ],
        };
        let summary = format_frame_transition_summary(Some(&trace));
        assert!(summary.contains("frame="));
        assert!(summary.contains("created"));
        assert!(summary.contains("progressed"));
    }

    #[test]
    fn frame_transition_summary_none_is_empty() {
        let summary = format_frame_transition_summary(None);
        assert!(summary.is_empty());
    }

    #[test]
    fn frame_transition_in_active_action_summary() {
        let outcome = DecisionOutcome::ActiveAction {
            action_def_id: ActionDefId(0),
            action_name: "travel".to_string(),
            interrupt: InterruptTrace {
                decision: InterruptDecision::NoInterrupt,
                top_challenger: None,
                top_challenger_comparison: None,
            },
            frame_transition: Some(FrameTransitionTrace {
                transitions: vec![FrameTransitionKind::Progressed { tick: Tick(5) }],
            }),
        };
        let summary = outcome.summary();
        assert!(summary.contains("ACTIVE"));
        assert!(summary.contains("frame="));
        assert!(summary.contains("progressed"));
    }

    #[test]
    fn frame_transition_in_planning_summary() {
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                fully_blocked_desires: vec![],
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                damped: Vec::new(),
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_bandit: vec![],
                omitted_social: vec![],
                omitted_violation_detection: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            discrepancy_trace: vec![],
            exhaustion_snapshot: vec![],
            frame_transition: Some(FrameTransitionTrace {
                transitions: vec![FrameTransitionKind::Created {
                    goal: GoalKey::new(GoalKind::Sleep),
                    domain_tag: IntentionDomainTag::Travel,
                    patience_limit: 30,
                    assumptions_count: 1,
                }],
            }),
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        }));
        let summary = outcome.summary();
        assert!(summary.contains("PLAN"));
        assert!(summary.contains("frame="));
        assert!(summary.contains("created"));
    }

    #[test]
    fn zero_cost_when_no_transitions() {
        // When frame_transition is None, summary should not contain "frame="
        let outcome = DecisionOutcome::ActiveAction {
            action_def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            interrupt: InterruptTrace {
                decision: InterruptDecision::NoInterrupt,
                top_challenger: None,
                top_challenger_comparison: None,
            },
            frame_transition: None,
        };
        let summary = outcome.summary();
        assert!(!summary.contains("frame="));
    }

    #[test]
    fn knowledge_path_format_perception_source() {
        use worldwake_core::PerceptionSource;

        assert_eq!(
            format_perception_source(&PerceptionSource::DirectObservation),
            "DirectObservation"
        );
        let from = entity(42);
        assert_eq!(
            format_perception_source(&PerceptionSource::Report { from, chain_len: 2 }),
            format!("Report(from={from:?}, chain=2)")
        );
        assert_eq!(
            format_perception_source(&PerceptionSource::Rumor { chain_len: 3 }),
            "Rumor(chain=3)"
        );
        assert_eq!(
            format_perception_source(&PerceptionSource::Inference),
            "Inference"
        );
    }

    #[test]
    fn knowledge_path_format_institutional_source() {
        use worldwake_core::{InstitutionalKnowledgeSource, RecordEntryId};

        assert_eq!(
            format_institutional_knowledge_source(&InstitutionalKnowledgeSource::WitnessedEvent),
            "WitnessedEvent"
        );
        let from = entity(7);
        assert_eq!(
            format_institutional_knowledge_source(&InstitutionalKnowledgeSource::Report {
                from,
                chain_len: 1
            }),
            format!("Report(from={from:?}, chain=1)")
        );
        let record = entity(20);
        assert_eq!(
            format_institutional_knowledge_source(
                &InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: RecordEntryId(5),
                }
            ),
            format!("RecordConsultation(record={record:?}, entry=5)")
        );
        assert_eq!(
            format_institutional_knowledge_source(&InstitutionalKnowledgeSource::SelfDeclaration),
            "SelfDeclaration"
        );
    }

    #[test]
    fn dump_agent_renders_knowledge_path_when_nonempty() {
        use crate::knowledge_path::{
            BeliefAspect, BeliefProvenance, KnowledgePath, SelfKnowledgeProvenance,
        };
        use worldwake_core::{CommodityKind, HomeostaticNeedId, PerceptionSource, Permille};

        let mut sink = DecisionTraceSink::new();
        let agent = entity(1);
        let seller = entity(10);
        let place = entity(20);

        let goal_key = GoalKey {
            kind: GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            commodity: Some(CommodityKind::Apple),
            entity: Some(seller),
            place: Some(place),
        };
        let evidence = CandidateEvidenceTrace {
            opportunity: default_opportunity(goal_key),
            contributors: vec![CandidateEvidenceContributor {
                kind: CandidateEvidenceKind::Seller,
                place,
                entity: seller,
            }],
            exclusions: vec![],
            knowledge_path: KnowledgePath {
                self_knowledge: vec![SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Hunger,
                    permille: Permille::new(900).unwrap(),
                }],
                entity_beliefs: vec![BeliefProvenance {
                    subject: seller,
                    aspect: BeliefAspect::HasCommodity {
                        commodity: CommodityKind::Apple,
                    },
                    source: PerceptionSource::DirectObservation,
                    observed_tick: Tick(8),
                }],
                institutional_beliefs: vec![],
            },
            legality: None,
            pursuit: None,
            artifact_axes: None,
        };

        let trace = AgentDecisionTrace {
            agent,
            tick: Tick(5),
            opportunity_compiler_load: None,
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: crate::DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: vec![default_opportunity(goal_key)],
                    evidence: vec![evidence],
                    fully_blocked_desires: vec![],
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: vec![RankedGoalSummary {
                        opportunity: default_opportunity(goal_key),
                        priority_class: GoalPriorityClass::Medium,
                        motive_score: 100,
                        provenance: None,
                        source_reliability_discount: None,
                        competition_discount: None,
                        source_composite: None,
                        feasibility: FeasibilityHint::Likely,
                        acquisition_quantity: None,
                        artifact_axes: None,
                    }],
                    top_ranked_comparison: None,
                    suppressed: vec![],
                    damped: Vec::new(),
                    zero_motive: vec![],
                    omitted_political: vec![],
                    omitted_bandit: vec![],
                    omitted_social: vec![],
                    omitted_violation_detection: vec![],
                },
                planning: PlanSearchTrace {
                    attempts: vec![],
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: None,
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: None,
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: vec![],
                discrepancy_trace: vec![],
                exhaustion_snapshot: vec![],
                frame_transition: None,
                patrol_route: PatrolRouteSnapshotTrace::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        };
        sink.record(trace);

        // Capture stderr output by calling format_knowledge_path directly
        // (dump_agent writes to stderr which is harder to capture in test).
        let DecisionOutcome::Planning(planning) = &sink.traces()[0].outcome else {
            panic!("expected Planning")
        };
        let ev = &planning.candidates.evidence[0];
        let lines = format_knowledge_path(&ev.knowledge_path);
        assert!(!lines.is_empty(), "knowledge path should produce output");
        assert_eq!(lines[0], "    Knowledge path:");
        assert!(lines[1].contains("self: NeedLevel(Hunger, 900 permille)"));
        assert!(lines[2].contains("belief:"));
        assert!(lines[2].contains("DirectObservation"));
        assert!(lines[2].contains("tick 8"));
    }

    #[test]
    fn dump_agent_omits_knowledge_path_when_empty() {
        use crate::knowledge_path::KnowledgePath;

        let kp = KnowledgePath::default();
        let lines = format_knowledge_path(&kp);
        assert!(
            lines.is_empty(),
            "empty knowledge path should produce no output"
        );
    }

    #[test]
    fn selected_plan_format_tolerates_missing_binding_strictness_snapshot() {
        let summary = PlannedStepSummary {
            action_def_id: ActionDefId(1),
            action_name: "sleep".to_string(),
            op_kind: PlannerOpKind::Sleep,
            targets: vec![],
            estimated_ticks: 2,
            binding_strictness: None,
        };
        let selected_plan = SelectedPlanTrace {
            steps: vec![summary.clone()],
            terminal_kind: PlanTerminalKind::GoalSatisfied,
            next_step_index: Some(0),
            next_step: Some(summary),
            search_provenance: None,
            primary_motive: 100,
            total_value: 100,
            side_benefits: vec![],
        };

        let formatted = format_selected_plan(&selected_plan);

        assert!(formatted.contains("GoalSatisfied"));
        assert!(formatted.contains("next_step=Sleep"));
    }

    // ── Pursuit trace tests ───────────────────────────────────────

    #[test]
    fn test_pursuit_trace_emitted_candidate() {
        let target = entity(10);
        let place = entity(20);
        let pd = PursuitDiagnostic {
            target,
            believed_place: Some(place),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(800).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: Some(3),
            max_travel_ticks: 10,
            omission: None,
        };

        // Emitted candidate: omission is None
        assert!(pd.omission.is_none());
        assert_eq!(pd.target, target);
        assert_eq!(pd.believed_place, Some(place));
        assert_eq!(pd.source, Some(PerceptionSource::DirectObservation));
        assert_eq!(pd.observed_tick, Some(Tick(5)));
        assert_eq!(pd.derived_confidence.unwrap().value(), 800);
        assert_eq!(pd.route_cost, Some(3));
        assert_eq!(pd.max_travel_ticks, 10);

        // Verify it can be stored on CandidateEvidenceTrace
        let evidence = CandidateEvidenceTrace {
            opportunity: OpportunityKey {
                goal_key: GoalKey::from(GoalKind::EngageHostile { target }),
                anchor: OpportunityAnchor::Entity(target),
            },
            contributors: Vec::new(),
            exclusions: Vec::new(),
            knowledge_path: crate::knowledge_path::KnowledgePath::default(),
            legality: None,
            pursuit: Some(pd),
            artifact_axes: None,
        };
        assert!(evidence.pursuit.is_some());
        assert!(evidence.pursuit.as_ref().unwrap().omission.is_none());
    }

    #[test]
    fn test_pursuit_trace_omitted_candidate() {
        let target = entity(10);
        let place = entity(20);

        // Low confidence omission
        let pd_low = PursuitDiagnostic {
            target,
            believed_place: Some(place),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(200).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: None,
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::LowConfidence),
        };
        assert_eq!(pd_low.omission, Some(PursuitOmissionReason::LowConfidence));

        // Unknown place omission (no belief data)
        let pd_unknown = PursuitDiagnostic {
            target,
            believed_place: None,
            source: None,
            observed_tick: None,
            derived_confidence: None,
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: None,
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::UnknownPlace),
        };
        assert_eq!(
            pd_unknown.omission,
            Some(PursuitOmissionReason::UnknownPlace)
        );
        assert!(pd_unknown.believed_place.is_none());

        // Over-range omission
        let pd_over = PursuitDiagnostic {
            target,
            believed_place: Some(place),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(800).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: Some(15),
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::OverRange),
        };
        assert_eq!(pd_over.omission, Some(PursuitOmissionReason::OverRange));
        assert_eq!(pd_over.route_cost, Some(15));

        // Blocked omission
        let pd_blocked = PursuitDiagnostic {
            target,
            believed_place: Some(place),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(800).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: Some(3),
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::Blocked),
        };
        assert_eq!(pd_blocked.omission, Some(PursuitOmissionReason::Blocked));

        // Unreachable omission
        let pd_unreachable = PursuitDiagnostic {
            target,
            believed_place: Some(place),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(800).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: None,
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::Unreachable),
        };
        assert_eq!(
            pd_unreachable.omission,
            Some(PursuitOmissionReason::Unreachable)
        );
    }

    #[test]
    fn test_pursuit_invalidation_trace() {
        // Verify all invalidation reasons can be stored and compared
        let reasons = [
            PursuitInvalidationReason::NoProfile,
            PursuitInvalidationReason::NoBelief,
            PursuitInvalidationReason::TargetDead,
            PursuitInvalidationReason::PlaceUnknown,
            PursuitInvalidationReason::CoLocated,
            PursuitInvalidationReason::PlaceChanged,
            PursuitInvalidationReason::ConfidenceDecayed,
        ];
        for reason in reasons {
            let opt: Option<PursuitInvalidationReason> = Some(reason);
            assert_eq!(opt, Some(reason));
        }

        // Verify format_pursuit_diagnostic does not panic on emitted trace
        let pd_emitted = PursuitDiagnostic {
            target: entity(10),
            believed_place: Some(entity(20)),
            source: Some(PerceptionSource::DirectObservation),
            observed_tick: Some(Tick(5)),
            derived_confidence: Some(Permille::new(800).unwrap()),
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: Some(3),
            max_travel_ticks: 10,
            omission: None,
        };
        format_pursuit_diagnostic(&pd_emitted);

        // Verify format_pursuit_diagnostic does not panic on omitted trace
        let diagnostic_with_omission = PursuitDiagnostic {
            target: entity(10),
            believed_place: None,
            source: None,
            observed_tick: None,
            derived_confidence: None,
            min_confidence_threshold: Permille::new(500).unwrap(),
            route_cost: None,
            max_travel_ticks: 10,
            omission: Some(PursuitOmissionReason::UnknownPlace),
        };
        format_pursuit_diagnostic(&diagnostic_with_omission);
    }
}
