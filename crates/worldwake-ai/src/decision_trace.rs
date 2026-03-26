//! Structured decision trace data model and collection sink.
//!
//! Records per-agent per-tick decision reasoning for diagnostic
//! and test query purposes. See spec S08 for design rationale.

use std::fmt::Write as _;
use worldwake_core::{
    ActionDefId, BlockingFact, CommodityKind, EntityId, FrameClearReason, GoalKey,
    InstitutionalClaim, InstitutionalKnowledgeSource, IntentionDomainTag, PerceptionSource,
    SuspensionReason, TellTopic, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionStartFailureReason, ResolvedRequestTrace, TellTopicOmissionReason,
};

use crate::feasibility::FeasibilityHint;
use crate::goal_model::{GoalPriorityClass, RankedGoalProvenance};
use crate::goal_switching::GoalSwitchKind;
use crate::interrupts::InterruptDecision;
use crate::knowledge_path::{
    BeliefAspect, BeliefProvenance, InstitutionalBeliefProvenance, KnowledgePath,
    SelfKnowledgeProvenance,
};
use crate::planner_ops::{PlanTerminalKind, PlannerOpKind};
use crate::planner_duration_contract::PlannerDurationDependency;
use crate::ranking::RankedGoalComparison;
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
    },
}

/// Collected frame lifecycle events for one agent-tick.
#[derive(Clone, Debug)]
pub struct FrameTransitionTrace {
    pub transitions: Vec<FrameTransitionKind>,
}

// ── Top-Level Record ────────────────────────────────────────────

/// One complete decision record for one agent at one tick.
#[derive(Clone, Debug)]
pub struct AgentDecisionTrace {
    pub agent: EntityId,
    pub tick: Tick,
    pub outcome: DecisionOutcome,
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
                    .selected
                    .as_ref()
                    .map_or_else(|| "none".to_string(), |g| format!("{:?}", g.kind));
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
                let ranking_suffix = planning
                    .candidates
                    .top_ranked_comparison
                    .as_ref()
                    .map_or_else(String::new, format_ranked_goal_comparison_summary);
                let unknown_suffix = if planning.unknown_blockers.is_empty() {
                    String::new()
                } else {
                    format!(", unknown_blockers={}", planning.unknown_blockers.len())
                };
                let frame_suffix =
                    format_frame_transition_summary(planning.frame_transition.as_ref());
                let dirty = planning.dirty.display_names();
                format!(
                    "PLAN (dirty: {dirty}): selected={selected}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}{selected_provenance}{selected_feasibility}{ranking_suffix}{unknown_suffix}{frame_suffix}"
                )
            }
        }
    }
}

// ── Planning Pipeline ───────────────────────────────────────────

/// Full trace of the planning pipeline for one agent-tick.
#[derive(Clone, Debug)]
pub struct PlanningPipelineTrace {
    pub dirty: crate::DirtySet,
    /// When true, the existing plan was revalidated instead of replanning from
    /// scratch. This happens when `dirty.is_snapshot_only()` is true
    /// and the current plan's next step passes revalidation.
    pub plan_continued: bool,
    pub candidates: CandidateTrace,
    pub planning: PlanSearchTrace,
    pub selection: SelectionTrace,
    pub execution: ExecutionTrace,
    /// Action start failures from the previous tick's `BestEffort` inputs,
    /// drained from the `Scheduler` for this agent.
    pub action_start_failures: Vec<ActionStartFailureSummary>,
    /// Active `BlockingFact::Unknown` blockers in `BlockedIntentMemory` at
    /// trace construction time. Derived view for debuggability (P27).
    pub unknown_blockers: Vec<UnknownBlockerTrace>,
    /// Frame lifecycle events recorded during this tick (P27).
    pub frame_transition: Option<FrameTransitionTrace>,
}

/// Summary of an action start failure for trace output.
#[derive(Clone, Debug)]
pub struct ActionStartFailureSummary {
    pub tick: Tick,
    pub def_id: ActionDefId,
    pub request: ResolvedRequestTrace,
    pub reason: ActionStartFailureReason,
}

/// Diagnostic trace for `BlockingFact::Unknown` blockers active during planning.
/// Derived from `BlockedIntentMemory` at trace construction time (P25: derived view).
#[derive(Clone, Debug)]
pub struct UnknownBlockerTrace {
    pub goal_key: GoalKey,
    pub failed_action_def: ActionDefId,
    pub op_kind: PlannerOpKind,
    pub target: Option<EntityId>,
    pub place: Option<EntityId>,
}

// ── Stage 1: Candidate Generation + Ranking ─────────────────────

/// Trace of candidate generation and ranking.
#[derive(Clone, Debug)]
pub struct CandidateTrace {
    /// All grounded goal keys generated (before suppression/zero-motive filter).
    pub generated: Vec<GoalKey>,
    /// Typed candidate-evidence provenance keyed by grounded goal.
    pub evidence: Vec<CandidateEvidenceTrace>,
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoalSummary>,
    /// Why the highest-ranked goal beat the immediate runner-up, when at least
    /// two ranked candidates exist.
    pub top_ranked_comparison: Option<RankedGoalComparison>,
    /// Goals suppressed by situational conditions.
    pub suppressed: Vec<GoalKey>,
    /// Goals filtered by zero motive score.
    pub zero_motive: Vec<GoalKey>,
    /// Political goals omitted before generation due to hard gates.
    pub omitted_political: Vec<PoliticalCandidateOmission>,
    /// Social goals omitted before generation due to resend suppression.
    pub omitted_social: Vec<SocialCandidateOmission>,
}

/// Political goal families that can be omitted before candidate emission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoliticalGoalFamily {
    ClaimOffice,
    SupportCandidateForOffice,
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
}

/// Diagnostic record for a political goal omitted before generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoliticalCandidateOmission {
    pub family: PoliticalGoalFamily,
    pub office: EntityId,
    pub candidate: Option<EntityId>,
    pub reason: PoliticalCandidateOmissionReason,
}

/// Diagnostic record for a social goal omitted before generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocialCandidateOmission {
    pub listener: EntityId,
    pub topic: TellTopic,
    pub reason: TellTopicOmissionReason,
}

/// Summary of a ranked goal for trace output.
#[derive(Clone, Debug)]
pub struct RankedGoalSummary {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub feasibility: FeasibilityHint,
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
    pub goal: GoalKey,
    pub contributors: Vec<CandidateEvidenceContributor>,
    pub exclusions: Vec<CandidateEvidenceExclusion>,
    /// Knowledge path: which beliefs motivated this candidate and where they came from.
    /// Empty when tracing is disabled.
    pub knowledge_path: KnowledgePath,
}

// ── Stage 2: Plan Search ────────────────────────────────────────

/// Trace of plan search attempts across candidates.
#[derive(Clone, Debug)]
pub struct PlanSearchTrace {
    /// One entry per candidate that was planned (top N by budget).
    pub attempts: Vec<PlanAttemptTrace>,
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
    BlockedFacilityUse {
        facility: EntityId,
        intended_action: ActionDefId,
    },
    PlaceBlocker {
        place: Option<EntityId>,
        blocking_fact: BlockingFact,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootCandidateTrace {
    pub def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: Option<PlannerOpKind>,
    pub authoritative_targets: Vec<EntityId>,
    pub planner_only: bool,
    pub payload_status: RootCandidatePayloadStatus,
    pub outcome: RootCandidateOutcome,
}

/// Why a relevant root operator never produced a concrete root candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootOperatorOmissionReason {
    NoMatchingActionDef,
    NoAffordanceOrSynthesisPath,
    SynthesisUnsupportedGoalOp,
    SynthesisTargetDerivationFailed,
}

/// Structured omission provenance for one relevant operator at the root boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootOperatorOmissionTrace {
    pub op_kind: PlannerOpKind,
    pub reason: RootOperatorOmissionReason,
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
    /// Travel-pruning facts captured before successor construction when the
    /// expansion had spatially guided travel choices.
    pub travel_pruning: Option<TravelPruningTrace>,
    /// Concrete goal-relevant / prerequisite guidance surfaces for this
    /// expansion boundary, when any exist.
    pub prerequisite_guidance: Option<PrerequisiteGuidanceTrace>,
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
    pub remaining_travel_ticks: u32,
}

/// Structured summary of spatial pruning at one expansion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TravelPruningTrace {
    pub current_place: EntityId,
    pub current_remaining_travel_ticks: u32,
    pub retained: Vec<TravelSuccessorTrace>,
    pub pruned: Vec<TravelSuccessorTrace>,
}

/// Trace of a single plan search attempt for one goal.
#[derive(Clone, Debug)]
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub outcome: PlanSearchOutcome,
    pub binding_rejections: Vec<BindingRejection>,
    /// Per-expansion summaries. Empty when tracing is disabled.
    pub expansion_summaries: Vec<SearchExpansionSummary>,
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
}

// ── Stage 3: Plan Selection ─────────────────────────────────────

/// Trace of plan selection and goal switching.
#[derive(Clone, Debug)]
pub struct SelectionTrace {
    /// The goal/plan that was selected (None if no plans available).
    pub selected: Option<GoalKey>,
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
}

/// Compact planner-owned provenance for the selected fresh search result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPlanSearchProvenance {
    pub expansions_used: u16,
    pub root_remaining_travel_ticks: u32,
    pub root_travel_pruning: Option<TravelPruningTrace>,
}

/// Provenance for the final selected plan surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedPlanSource {
    SearchSelection,
    RetainedCurrentPlan,
    SnapshotContinuation,
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
    SameGoalBranchReplanned,
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

// ── Collection Sink ─────────────────────────────────────────────

/// Append-only collection of decision traces with query helpers.
///
/// All query methods compute on the fly from the internal `Vec` —
/// no derived state is stored.
#[derive(Clone, Debug)]
pub struct DecisionTraceSink {
    traces: Vec<AgentDecisionTrace>,
}

impl DecisionTraceSink {
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }

    pub fn record(&mut self, trace: AgentDecisionTrace) {
        self.traces.push(trace);
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
                for ev in &planning.candidates.evidence {
                    let feasibility = planning
                        .candidates
                        .ranked
                        .iter()
                        .find(|r| r.goal == ev.goal)
                        .map_or(FeasibilityHint::Uncertain, |r| r.feasibility);
                    eprintln!(
                        "  Candidate: {:?} [feasibility={feasibility:?}]",
                        ev.goal.kind
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
                }
            }
        }
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
        .ranked
        .iter()
        .position(|candidate| candidate.goal == goal_key)
    {
        return GoalTraceStatus::Ranked {
            rank,
            selected: planning.selection.selected == Some(goal_key),
        };
    }
    if planning.candidates.generated.contains(&goal_key) {
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
        _ => None,
    })
}

fn omitted_social_reason_for_goal(
    omissions: &[SocialCandidateOmission],
    goal: &crate::GoalKind,
) -> Option<TellTopicOmissionReason> {
    omissions.iter().find_map(|omission| match goal {
        crate::GoalKind::ShareBelief { listener, topic }
            if omission.listener == *listener && omission.topic == *topic =>
        {
            Some(omission.reason)
        }
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
                .selected
                .as_ref()
                .map_or_else(|| "none".to_string(), |g| format!("{:?}", g.kind));
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
            let ranking = planning
                .candidates
                .top_ranked_comparison
                .as_ref()
                .map_or_else(String::new, format_ranked_goal_comparison_summary);
            let dirty = planning.dirty.display_names();
            let mut out = format!(
                "PLAN (dirty: {dirty}): selected={selected}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}{selected_provenance}{selected_feasibility}{ranking}"
            );
            for attempt in &planning.planning.attempts {
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
                for exp in &attempt.expansion_summaries {
                    let satisfied_tag = if exp.found_goal_satisfied {
                        " satisfied"
                    } else {
                        ""
                    };
                    let _ = write!(
                        out,
                        "\n  search expansion d={}: {} candidates, {} skipped, {} terminal{}, {}→{} beam",
                        exp.depth,
                        exp.candidates_generated,
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
            if !planning.unknown_blockers.is_empty() {
                let _ = write!(out, "\n  Unknown blockers active:");
                for ub in &planning.unknown_blockers {
                    let def_name = action_defs
                        .get(ub.failed_action_def)
                        .map_or("unknown", |d| d.name.as_str());
                    let _ = write!(
                        out,
                        "\n    goal={:?} action={def_name} op={:?} place={:?}",
                        ub.goal_key.kind, ub.op_kind, ub.place,
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
    let selected = planning.selection.selected?;
    planning
        .candidates
        .ranked
        .iter()
        .find(|summary| summary.goal == selected)
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
            let adjustment = provenance
                .adjustment
                .map_or_else(|| "none".to_string(), |adjustment| format!("{adjustment:?}"));
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
        ", ranking={:?} {:?}>{:?}",
        comparison.decisive_dimension, comparison.winner.kind, comparison.loser.kind
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
        "{:?}[steps={}, next_index={:?}, next_step={next_step}, path={step_kinds}, search={search}]",
        selected_plan.terminal_kind,
        selected_plan.steps.len(),
        selected_plan.next_step_index,
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
                        "{:?}@{}",
                        successor.destination, successor.remaining_travel_ticks
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let pruned = trace
                .pruned
                .iter()
                .map(|successor| {
                    format!(
                        "{:?}@{}",
                        successor.destination, successor.remaining_travel_ticks
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
    format!(
        "expansions={}, root_remaining={}, pruning={pruning}",
        provenance.expansions_used, provenance.root_remaining_travel_ticks
    )
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
        FrameTransitionKind::Cleared { reason } => {
            format!("FRAME_CLEARED: reason={reason:?}")
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
    use worldwake_core::{GoalKind, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn dead_trace(agent: EntityId, tick: Tick) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent,
            tick,
            outcome: DecisionOutcome::Dead,
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
        omitted_social: Vec<SocialCandidateOmission>,
    ) -> AgentDecisionTrace {
        AgentDecisionTrace {
            agent: entity(1),
            tick,
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                dirty: crate::DirtySet::default(),
                plan_continued,
                candidates: CandidateTrace {
                    generated,
                    evidence: Vec::new(),
                    ranked,
                    top_ranked_comparison: None,
                    suppressed,
                    zero_motive,
                    omitted_political,
                    omitted_social,
                },
                planning: PlanSearchTrace {
                    attempts: Vec::new(),
                },
                selection: SelectionTrace {
                    selected,
                    selected_plan: None,
                    selected_plan_source,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                },
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::new(),
                unknown_blockers: Vec::new(),
                frame_transition: None,
            })),
        }
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
                    goal: GoalKey::from(&selected_goal),
                    priority_class: GoalPriorityClass::High,
                    motive_score: 900,
                    provenance: None,
                    feasibility: FeasibilityHint::Uncertain,
                },
                RankedGoalSummary {
                    goal: GoalKey::from(&outranked_goal),
                    priority_class: GoalPriorityClass::Medium,
                    motive_score: 600,
                    provenance: None,
                    feasibility: FeasibilityHint::Uncertain,
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
    fn goal_status_reports_social_direct_observability_omission_reason() {
        let listener = entity(10);
        let subject = entity(11);
        let share_goal = GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
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
                goal: GoalKey::from(&goal),
                priority_class: GoalPriorityClass::Medium,
                motive_score: 700,
                provenance: None,
                feasibility: FeasibilityHint::Uncertain,
            }],
            Some(GoalKey::from(&goal)),
            Some(SelectedPlanSource::SearchSelection),
            false,
            Vec::new(),
            Vec::new(),
        ));
        sink.record(goal_trace(
            Tick(2),
            vec![GoalKey::from(&goal)],
            Vec::new(),
            Vec::new(),
            vec![RankedGoalSummary {
                goal: GoalKey::from(&goal),
                priority_class: GoalPriorityClass::Medium,
                motive_score: 700,
                provenance: None,
                feasibility: FeasibilityHint::Uncertain,
            }],
            Some(GoalKey::from(&goal)),
            Some(SelectedPlanSource::SnapshotContinuation),
            true,
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
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                ranked: vec![RankedGoalSummary {
                    goal: GoalKey::new(GoalKind::Sleep),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 800,
                    provenance: None,
                    feasibility: FeasibilityHint::Uncertain,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace { attempts: vec![] },
            selection: SelectionTrace {
                selected: Some(GoalKey::new(GoalKind::Sleep)),
                selected_plan: Some(SelectedPlanTrace {
                    steps: vec![PlannedStepSummary {
                        action_def_id: ActionDefId(1),
                        action_name: "sleep".to_string(),
                        op_kind: PlannerOpKind::Sleep,
                        targets: vec![],
                        estimated_ticks: 2,
                    }],
                    terminal_kind: PlanTerminalKind::GoalSatisfied,
                    next_step_index: Some(0),
                    next_step: Some(PlannedStepSummary {
                        action_def_id: ActionDefId(1),
                        action_name: "sleep".to_string(),
                        op_kind: PlannerOpKind::Sleep,
                        targets: vec![],
                        estimated_ticks: 2,
                    }),
                    search_provenance: Some(SelectedPlanSearchProvenance {
                        expansions_used: 3,
                        root_remaining_travel_ticks: 7,
                        root_travel_pruning: Some(TravelPruningTrace {
                            current_place: entity(11),
                            current_remaining_travel_ticks: 7,
                            retained: vec![TravelSuccessorTrace {
                                destination: entity(12),
                                remaining_travel_ticks: 5,
                            }],
                            pruned: vec![TravelSuccessorTrace {
                                destination: entity(13),
                                remaining_travel_ticks: 9,
                            }],
                        }),
                    }),
                }),
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: None,
        }));
        let summary = outcome.summary();
        assert!(summary.contains("PLAN"));
        assert!(summary.contains("candidates=1"));
        assert!(summary.contains("plans_found=0"));
        assert!(summary.contains("Sleep"));
        assert!(summary.contains("SearchSelection"));
        assert!(summary.contains("GoalSatisfied"));
        assert!(summary.contains("Sleep]") || summary.contains("path=Sleep"));
        assert!(summary.contains("expansions=3"));
        assert!(summary.contains("root_remaining=7"));
        assert!(summary.contains("pruned=["));
    }

    #[test]
    fn summary_planning_includes_ranking_comparison() {
        use crate::ranking::{RankedGoalComparison, RankedGoalComparisonDimension};
        use worldwake_core::GoalKind;

        let winner = GoalKey::new(GoalKind::Sleep);
        let loser = GoalKey::new(GoalKind::Wash);
        let outcome = DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![winner, loser],
                evidence: vec![],
                ranked: vec![
                    RankedGoalSummary {
                        goal: winner,
                        priority_class: GoalPriorityClass::Critical,
                        motive_score: 800,
                        provenance: None,
                        feasibility: FeasibilityHint::Likely,
                    },
                    RankedGoalSummary {
                        goal: loser,
                        priority_class: GoalPriorityClass::Critical,
                        motive_score: 600,
                        provenance: None,
                        feasibility: FeasibilityHint::Likely,
                    },
                ],
                top_ranked_comparison: Some(RankedGoalComparison {
                    winner,
                    loser,
                    decisive_dimension: RankedGoalComparisonDimension::MotiveScore,
                }),
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace { attempts: vec![] },
            selection: SelectionTrace {
                selected: Some(winner),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: None,
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
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![GoalKey::new(GoalKind::ReduceDanger)],
                evidence: vec![],
                ranked: vec![RankedGoalSummary {
                    goal: GoalKey::new(GoalKind::ReduceDanger),
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
                    feasibility: FeasibilityHint::Uncertain,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace { attempts: vec![] },
            selection: SelectionTrace {
                selected: Some(GoalKey::new(GoalKind::ReduceDanger)),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: None,
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
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![GoalKey::new(GoalKind::ConsumeOwnedCommodity {
                    commodity: worldwake_core::CommodityKind::Bread,
                })],
                evidence: vec![],
                ranked: vec![RankedGoalSummary {
                    goal: GoalKey::new(GoalKind::ConsumeOwnedCommodity {
                        commodity: worldwake_core::CommodityKind::Bread,
                    }),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 380_000,
                    provenance: Some(RankedGoalProvenance::Drive(
                        crate::RankedDriveGoalProvenance {
                            base_priority_class: GoalPriorityClass::High,
                            final_priority_class: GoalPriorityClass::Critical,
                            adjustment: Some(
                                crate::RankedPriorityAdjustment::ClottedWoundRecoveryPromotion,
                            ),
                            motive_inputs: vec![crate::RankedDriveMotiveInput {
                                drive: crate::RankedDriveKind::Hunger,
                                pressure: worldwake_core::Permille::new(760).unwrap(),
                                weight: worldwake_core::Permille::new(500).unwrap(),
                                score: 380_000,
                                recovery_relevant: true,
                            }],
                        },
                    )),
                    feasibility: FeasibilityHint::Uncertain,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace { attempts: vec![] },
            selection: SelectionTrace {
                selected: Some(GoalKey::new(GoalKind::ConsumeOwnedCommodity {
                    commodity: worldwake_core::CommodityKind::Bread,
                })),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: None,
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
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![GoalKey::new(GoalKind::ClaimOffice { office: entity(4) })],
                evidence: vec![],
                ranked: vec![RankedGoalSummary {
                    goal: GoalKey::new(GoalKind::ClaimOffice { office: entity(4) }),
                    priority_class: GoalPriorityClass::High,
                    motive_score: 400,
                    provenance: None,
                    feasibility: FeasibilityHint::Uncertain,
                }],
                top_ranked_comparison: None,
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace {
                attempts: vec![PlanAttemptTrace {
                    goal: GoalKey::new(GoalKind::ClaimOffice { office: entity(4) }),
                    outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 1 },
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
                        travel_pruning: None,
                        prerequisite_guidance: None,
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
                        }],
                        root_omissions: vec![RootOperatorOmissionTrace {
                            op_kind: PlannerOpKind::PressForceClaim,
                            reason: RootOperatorOmissionReason::NoMatchingActionDef,
                        }],
                    }],
                }],
            },
            selection: SelectionTrace {
                selected: Some(GoalKey::new(GoalKind::ClaimOffice { office: entity(4) })),
                selected_plan: None,
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: None,
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
                domain: worldwake_sim::ActionDomain::Generic,
                actor_constraints: vec![],
                targets: vec![],
                preconditions: vec![],
                reservation_requirements: vec![],
                duration: worldwake_sim::DurationExpr::Fixed(std::num::NonZeroU32::new(1).unwrap()),
                body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
                interruptibility: worldwake_sim::Interruptibility::FreelyInterruptible,
                commit_conditions: vec![],
                visibility: worldwake_core::VisibilitySpec::SamePlace,
                causal_event_tags: std::collections::BTreeSet::new(),
                payload: worldwake_sim::ActionPayload::None,
                handler: worldwake_sim::ActionHandlerId(0),
            });
        }

        let summary = format_outcome(&outcome, &action_defs);
        assert!(summary.contains("root omission: PressForceClaim -> NoMatchingActionDef"));
        assert!(
            summary.contains(
                "root candidate: trade op=Trade payload=GoalSynthesized outcome=Skipped(DurationEstimateFailed { dependency: ActorTradeDisposition })"
            )
        );
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
            outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 5 },
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
            travel_pruning: Some(TravelPruningTrace {
                current_place: entity(1),
                current_remaining_travel_ticks: 4,
                retained: vec![TravelSuccessorTrace {
                    destination: entity(2),
                    remaining_travel_ticks: 2,
                }],
                pruned: vec![TravelSuccessorTrace {
                    destination: entity(3),
                    remaining_travel_ticks: 6,
                }],
            }),
            prerequisite_guidance: None,
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
        assert!(!summary.found_goal_satisfied);
        assert!(summary.travel_pruning.is_some());

        // Verify Debug is derived and non-empty.
        let debug = format!("{summary:?}");
        assert!(debug.contains("SearchExpansionSummary"));
        assert!(debug.contains("depth: 0"));
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
        };
        let formatted = format_frame_transition_kind(&kind);
        assert!(formatted.contains("FRAME_CLEARED"));
        assert!(formatted.contains("PatienceExhausted"));
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
            dirty: crate::DirtySet::NO_PLAN,
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                evidence: vec![],
                ranked: vec![],
                top_ranked_comparison: None,
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
                omitted_social: vec![],
            },
            planning: PlanSearchTrace { attempts: vec![] },
            selection: SelectionTrace {
                selected: None,
                selected_plan: None,
                selected_plan_source: None,
                goal_switch: None,
                previous_goal: None,
                plan_replacement: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
            unknown_blockers: vec![],
            frame_transition: Some(FrameTransitionTrace {
                transitions: vec![FrameTransitionKind::Created {
                    goal: GoalKey::new(GoalKind::Sleep),
                    domain_tag: IntentionDomainTag::Travel,
                    patience_limit: 30,
                    assumptions_count: 1,
                }],
            }),
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
            },
            commodity: Some(CommodityKind::Apple),
            entity: Some(seller),
            place: Some(place),
        };
        let evidence = CandidateEvidenceTrace {
            goal: goal_key,
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
        };

        let trace = AgentDecisionTrace {
            agent,
            tick: Tick(5),
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                dirty: crate::DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: vec![goal_key],
                    evidence: vec![evidence],
                    ranked: vec![RankedGoalSummary {
                        goal: goal_key,
                        priority_class: GoalPriorityClass::Medium,
                        motive_score: 100,
                        provenance: None,
                        feasibility: FeasibilityHint::Likely,
                    }],
                    top_ranked_comparison: None,
                    suppressed: vec![],
                    zero_motive: vec![],
                    omitted_political: vec![],
                    omitted_social: vec![],
                },
                planning: PlanSearchTrace { attempts: vec![] },
                selection: SelectionTrace {
                    selected: None,
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                },
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: vec![],
                unknown_blockers: vec![],
                frame_transition: None,
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
}
