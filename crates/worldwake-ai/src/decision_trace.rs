//! Structured decision trace data model and collection sink.
//!
//! Records per-agent per-tick decision reasoning for diagnostic
//! and test query purposes. See spec S08 for design rationale.

use std::fmt::Write as _;
use worldwake_core::{ActionDefId, EntityId, GoalKey, Tick};
use worldwake_sim::ActionDefRegistry;

use crate::goal_model::GoalPriorityClass;
use crate::goal_switching::GoalSwitchKind;
use crate::interrupts::InterruptDecision;
use crate::planner_ops::{PlanTerminalKind, PlannerOpKind};

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
pub enum DecisionOutcome {
    /// Agent is dead — no decision pipeline ran.
    Dead,

    /// Agent has an active action — interrupt evaluation ran.
    ActiveAction {
        action_def_id: ActionDefId,
        action_name: String,
        interrupt: InterruptTrace,
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
                ..
            } => {
                let decision = &interrupt.decision;
                format!("ACTIVE: {action_name} — interrupt: {decision:?}")
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
                format!(
                    "PLAN: selected={selected}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}"
                )
            }
        }
    }
}

// ── Planning Pipeline ───────────────────────────────────────────

/// Full trace of the planning pipeline for one agent-tick.
#[derive(Clone, Debug)]
pub struct PlanningPipelineTrace {
    pub dirty_reasons: Vec<DirtyReason>,
    /// When true, the existing plan was revalidated instead of replanning from
    /// scratch. This happens when `SnapshotChanged` is the only dirty reason
    /// and the current plan's next step passes revalidation.
    pub plan_continued: bool,
    pub candidates: CandidateTrace,
    pub planning: PlanSearchTrace,
    pub selection: SelectionTrace,
    pub execution: ExecutionTrace,
    /// Action start failures from the previous tick's `BestEffort` inputs,
    /// drained from the `Scheduler` for this agent.
    pub action_start_failures: Vec<ActionStartFailureSummary>,
}

/// Summary of an action start failure for trace output.
#[derive(Clone, Debug)]
pub struct ActionStartFailureSummary {
    pub tick: Tick,
    pub def_id: ActionDefId,
    pub reason: String,
}

// ── Stage 1: Candidate Generation + Ranking ─────────────────────

/// Trace of candidate generation and ranking.
#[derive(Clone, Debug)]
pub struct CandidateTrace {
    /// All grounded goal keys generated (before suppression/zero-motive filter).
    pub generated: Vec<GoalKey>,
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoalSummary>,
    /// Goals suppressed by situational conditions.
    pub suppressed: Vec<GoalKey>,
    /// Goals filtered by zero motive score.
    pub zero_motive: Vec<GoalKey>,
    /// Political goals omitted before generation due to hard gates.
    pub omitted_political: Vec<PoliticalCandidateOmission>,
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
    ActorNotEligible,
    CandidateNotEligible,
    AlreadyDeclaredSupport,
}

/// Diagnostic record for a political goal omitted before generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoliticalCandidateOmission {
    pub family: PoliticalGoalFamily,
    pub office: EntityId,
    pub candidate: Option<EntityId>,
    pub reason: PoliticalCandidateOmissionReason,
}

/// Summary of a ranked goal for trace output.
#[derive(Clone, Debug)]
pub struct RankedGoalSummary {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
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

/// Per-expansion summary recorded during plan search.
#[derive(Clone, Debug)]
pub struct SearchExpansionSummary {
    /// Depth (number of steps already in the node being expanded).
    pub depth: u8,
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
#[derive(Clone, Debug)]
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
}

// ── Dirty Reasons ───────────────────────────────────────────────

/// Why the planning pipeline was triggered (dirty flag reasons).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirtyReason {
    NoPlan,
    PlanFinished,
    ReplanSignal,
    QueueTransition,
    BlockerCleanup,
    SnapshotChanged,
    QueuePatienceExhausted,
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
        }
    }
}

/// Format a `DecisionOutcome` with action name resolution via the registry.
fn format_outcome(outcome: &DecisionOutcome, action_defs: &ActionDefRegistry) -> String {
    match outcome {
        DecisionOutcome::Dead => "DEAD — no decision".to_string(),
        DecisionOutcome::ActiveAction {
            action_def_id,
            action_name,
            interrupt,
            ..
        } => {
            let name = action_defs
                .get(*action_def_id)
                .map_or(action_name.as_str(), |d| d.name.as_str());
            let decision = &interrupt.decision;
            format!("ACTIVE: {name} — interrupt: {decision:?}")
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
            let mut out = format!(
                "PLAN: selected={selected}, source={provenance}, selected_plan={selected_plan}, candidates={candidates}, plans_found={plans_found}"
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
                }
            }
            out
        }
    }
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
    format!(
        "{:?}[steps={}, next_index={:?}, next_step={next_step}, path={step_kinds}]",
        selected_plan.terminal_kind,
        selected_plan.steps.len(),
        selected_plan.next_step_index,
    )
}

impl Default for DecisionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::Tick;

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
            },
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
            dirty_reasons: vec![DirtyReason::NoPlan],
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![],
                ranked: vec![RankedGoalSummary {
                    goal: GoalKey::new(GoalKind::Sleep),
                    priority_class: GoalPriorityClass::Critical,
                    motive_score: 800,
                }],
                suppressed: vec![],
                zero_motive: vec![],
                omitted_political: vec![],
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
                }),
                selected_plan_source: Some(SelectedPlanSource::SearchSelection),
                goal_switch: None,
                previous_goal: None,
            },
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: vec![],
        }));
        let summary = outcome.summary();
        assert!(summary.contains("PLAN"));
        assert!(summary.contains("candidates=1"));
        assert!(summary.contains("plans_found=0"));
        assert!(summary.contains("Sleep"));
        assert!(summary.contains("SearchSelection"));
        assert!(summary.contains("GoalSatisfied"));
        assert!(summary.contains("Sleep]") || summary.contains("path=Sleep"));
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
            candidates_generated: 12,
            candidates_skipped: 1,
            terminal_successors: 2,
            non_terminal_before_beam: 11,
            non_terminal_after_beam: 8,
            found_goal_satisfied: false,
        };
        assert_eq!(summary.depth, 0);
        assert_eq!(summary.candidates_generated, 12);
        assert_eq!(summary.candidates_skipped, 1);
        assert_eq!(summary.terminal_successors, 2);
        assert_eq!(summary.non_terminal_before_beam, 11);
        assert_eq!(summary.non_terminal_after_beam, 8);
        assert!(!summary.found_goal_satisfied);

        // Verify Debug is derived and non-empty.
        let debug = format!("{summary:?}");
        assert!(debug.contains("SearchExpansionSummary"));
        assert!(debug.contains("depth: 0"));
    }
}
