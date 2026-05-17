use std::collections::{BTreeMap, VecDeque};

use worldwake_ai::{
    AgentDecisionTrace, DecisionOutcome, SelectedPlanReplacementTrace, SelectionTrace,
};
use worldwake_core::EntityId;
use worldwake_sim::ActionTraceEvent;

pub const DEFAULT_TRACE_BUFFER_CAPACITY: usize = 50;

#[derive(Clone, Debug)]
pub struct AgentTraceBuffers {
    decisions: BTreeMap<EntityId, VecDeque<AgentDecisionTrace>>,
    actions: BTreeMap<EntityId, VecDeque<ActionTraceEvent>>,
    capacity: usize,
}

impl AgentTraceBuffers {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            decisions: BTreeMap::new(),
            actions: BTreeMap::new(),
            capacity,
        }
    }

    pub fn record_decision(&mut self, trace: AgentDecisionTrace) {
        push_capped(
            self.decisions.entry(trace.agent).or_default(),
            self.capacity,
            trace,
        );
    }

    pub fn record_action(&mut self, event: ActionTraceEvent) {
        push_capped(
            self.actions.entry(event.actor).or_default(),
            self.capacity,
            event,
        );
    }

    pub fn record_decisions<I>(&mut self, traces: I)
    where
        I: IntoIterator<Item = AgentDecisionTrace>,
    {
        for trace in traces {
            self.record_decision(trace);
        }
    }

    pub fn record_actions<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = ActionTraceEvent>,
    {
        for event in events {
            self.record_action(event);
        }
    }

    pub fn decisions_for(
        &self,
        agent: EntityId,
    ) -> impl DoubleEndedIterator<Item = &AgentDecisionTrace> {
        self.decisions
            .get(&agent)
            .into_iter()
            .flat_map(|traces| traces.iter())
    }

    pub fn actions_for(
        &self,
        agent: EntityId,
    ) -> impl DoubleEndedIterator<Item = &ActionTraceEvent> {
        self.actions
            .get(&agent)
            .into_iter()
            .flat_map(|events| events.iter())
    }

    pub fn decisions_for_newest_first(
        &self,
        agent: EntityId,
    ) -> impl Iterator<Item = &AgentDecisionTrace> {
        self.decisions_for(agent).rev()
    }

    pub fn actions_for_newest_first(
        &self,
        agent: EntityId,
    ) -> impl Iterator<Item = &ActionTraceEvent> {
        self.actions_for(agent).rev()
    }

    #[must_use]
    pub fn last_replan_reason(&self, agent: EntityId) -> Option<&AgentDecisionTrace> {
        self.decisions_for_newest_first(agent).find(|trace| {
            planning_selection(&trace.outcome).is_some_and(selection_has_replan_reason)
        })
    }

    #[must_use]
    pub fn last_replan_summary(&self, agent: EntityId) -> Option<String> {
        self.last_replan_reason(agent)
            .and_then(|trace| planning_selection(&trace.outcome))
            .and_then(replan_summary)
    }
}

impl Default for AgentTraceBuffers {
    fn default() -> Self {
        Self::new(DEFAULT_TRACE_BUFFER_CAPACITY)
    }
}

fn push_capped<T>(buffer: &mut VecDeque<T>, capacity: usize, item: T) {
    if capacity == 0 {
        return;
    }
    if buffer.len() == capacity {
        buffer.pop_front();
    }
    buffer.push_back(item);
}

fn planning_selection(outcome: &DecisionOutcome) -> Option<&SelectionTrace> {
    match outcome {
        DecisionOutcome::Planning(planning) => Some(&planning.selection),
        DecisionOutcome::Dead | DecisionOutcome::ActiveAction { .. } => None,
    }
}

fn selection_has_replan_reason(selection: &SelectionTrace) -> bool {
    selection.plan_replacement.is_some() || selection.goal_switch.is_some()
}

fn replan_summary(selection: &SelectionTrace) -> Option<String> {
    if let Some(replacement) = &selection.plan_replacement {
        return Some(format_replacement(replacement));
    }
    selection
        .goal_switch
        .as_ref()
        .map(|switch| format!("goal switch: {:?} -> {:?}", switch.from, switch.to))
}

fn format_replacement(replacement: &SelectedPlanReplacementTrace) -> String {
    format!(
        "{:?}: {:?} -> {:?}",
        replacement.kind, replacement.previous_goal, replacement.new_goal
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_ai::{
        CandidateTrace, DirtySet, ExecutionTrace, PlanSearchTrace, PlanningPipelineTrace,
        SelectedPlanReplacementKind, SelectionTrace,
    };
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, CommodityKind, CommodityPurpose, GoalKey, GoalKind, Tick,
    };
    use worldwake_sim::{ActionTraceEvent, ActionTraceKind};

    #[test]
    fn trace_buffers_capped_at_50() {
        let agent = entity(1);
        let mut buffers = AgentTraceBuffers::new(50);

        for tick in 0..100 {
            buffers.record_decision(dead_trace(agent, Tick(tick)));
        }

        let traces = buffers.decisions_for(agent).collect::<Vec<_>>();
        assert_eq!(traces.len(), 50);
        assert_eq!(traces.first().map(|trace| trace.tick), Some(Tick(50)));
        assert_eq!(traces.last().map(|trace| trace.tick), Some(Tick(99)));
        assert_eq!(
            buffers
                .decisions_for_newest_first(agent)
                .next()
                .map(|trace| trace.tick),
            Some(Tick(99))
        );
    }

    #[test]
    fn action_buffers_capped_and_partitioned_by_actor() {
        let actor = entity(1);
        let other = entity(2);
        let mut buffers = AgentTraceBuffers::new(2);

        buffers.record_action(action_event(actor, 1));
        buffers.record_action(action_event(other, 2));
        buffers.record_action(action_event(actor, 3));
        buffers.record_action(action_event(actor, 4));

        let actor_ticks = buffers
            .actions_for(actor)
            .map(|event| event.tick)
            .collect::<Vec<_>>();
        let other_ticks = buffers
            .actions_for(other)
            .map(|event| event.tick)
            .collect::<Vec<_>>();

        assert_eq!(actor_ticks, vec![Tick(3), Tick(4)]);
        assert_eq!(other_ticks, vec![Tick(2)]);
    }

    #[test]
    fn last_replan_reason_returns_most_recent() {
        let agent = entity(1);
        let mut buffers = AgentTraceBuffers::new(50);

        buffers.record_decision(dead_trace(agent, Tick(1)));
        buffers.record_decision(replan_trace(agent, Tick(3), GoalKind::Sleep));
        buffers.record_decision(dead_trace(agent, Tick(5)));
        buffers.record_decision(replan_trace(
            agent,
            Tick(7),
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
        ));

        let trace = buffers
            .last_replan_reason(agent)
            .expect("latest replan trace should be returned");
        assert_eq!(trace.tick, Tick(7));
        assert!(buffers
            .last_replan_summary(agent)
            .expect("summary should render")
            .contains("SameGoalSiblingReplaced"));
    }

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
            compiled_opportunities: Vec::new(),
            opportunity_compiler_load: None,
            snapshot_cache_counters: None,
            planning_state_cache_counters: None,
            repair_attempts: Vec::new(),
            causal_link_cap_hits: Vec::new(),
            outcome: DecisionOutcome::Dead,
        }
    }

    fn replan_trace(agent: EntityId, tick: Tick, goal_kind: GoalKind) -> AgentDecisionTrace {
        let goal = GoalKey::new(goal_kind);
        AgentDecisionTrace {
            agent,
            tick,
            compiled_opportunities: Vec::new(),
            opportunity_compiler_load: None,
            snapshot_cache_counters: None,
            planning_state_cache_counters: None,
            repair_attempts: Vec::new(),
            causal_link_cap_hits: Vec::new(),
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: DirtySet::default(),
                plan_continued: false,
                candidates: CandidateTrace {
                    generated: Vec::new(),
                    evidence: Vec::new(),
                    fully_blocked_desires: Vec::new(),
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: Vec::new(),
                    top_ranked_comparison: None,
                    suppressed: Vec::new(),
                    damped: Vec::new(),
                    zero_motive: Vec::new(),
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_testimony: Vec::new(),
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
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                frame_transition: None,
                patrol_route: Default::default(),
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
        }
    }

    fn action_event(actor: EntityId, tick: u64) -> ActionTraceEvent {
        ActionTraceEvent::new(
            Tick(tick),
            actor,
            ActionDefId(1),
            "wait".to_string(),
            ActionTraceKind::Started {
                targets: Vec::new(),
            },
        )
    }
}
