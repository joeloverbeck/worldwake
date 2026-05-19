use std::collections::{BTreeMap, BTreeSet};

use worldwake_core::{
    BeliefStatusTag, BlockerScope, ClaimantOutcome, DecisionEventPayload, Discrepancy, EventLog,
    EventTag, EventView, GoalKind, GoalRejectionReason, MethodSchemaId, OmissionReason,
    PercentileBucket, Permille, PlanInvalidationReason, ReplanReason, Tick, TopicScope,
};

use crate::{
    AgentDecisionTrace, CandidateSuppressionCategory, DecisionOutcome, PlanAttemptTrace,
    PlanSearchOutcome, RepairAttemptTrace,
};

use super::{
    BeliefMetrics, BlockerScopeVariantId, CoordinationMetrics, GoalPressureMetrics,
    MethodUsageCounts, PerformanceMetrics, PlanningMetrics, RevalidationRepairMetrics,
    ScenarioDiagnosticsReport,
};

#[must_use]
pub fn build_scenario_diagnostics(
    decision_traces: &[AgentDecisionTrace],
    plan_traces: &[PlanAttemptTrace],
    repair_traces: &[RepairAttemptTrace],
    event_log: &EventLog,
    tick_range: (Tick, Tick),
) -> ScenarioDiagnosticsReport {
    let mut builder = ScenarioDiagnosticsBuilder::default();

    for trace in decision_traces {
        builder.record_decision_trace(trace);
    }
    for trace in plan_traces {
        builder.record_plan_trace(trace);
    }
    for trace in repair_traces {
        builder.record_repair_trace(trace);
    }
    builder.record_event_log(event_log, tick_range);

    builder.finish(tick_range)
}

#[derive(Default)]
struct ScenarioDiagnosticsBuilder {
    candidates_emitted_by_kind: BTreeMap<GoalKind, u64>,
    candidates_emitted_by_slot: BTreeMap<crate::SlotKind, u64>,
    candidates_suppressed_by_category: BTreeMap<CandidateSuppressionCategory, u64>,
    top_k_not_planned: BTreeMap<GoalKind, u64>,
    planning_decision_count: u64,
    active_intention_continuation_count: u64,

    plan_attempts: u64,
    plan_attempts_by_kind: BTreeMap<GoalKind, u64>,
    budget_exhaustion_count: u64,
    frontier_exhaustion_count: u64,
    beam_truncated_count: u64,
    beam_candidate_count: u64,
    plan_depth_values: Vec<u64>,
    terminal_kind_distribution: BTreeMap<crate::PlanTerminalKindDiscriminant, u64>,
    heuristic_helpful_expansions: u64,
    heuristic_expansions: u64,
    method_usage: BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>,

    invalidation_reasons: BTreeMap<Discrepancy, u64>,
    repair_attempts: u64,
    repair_succeeded: u64,
    repair_budget_values: Vec<u64>,
    full_replan_count: u64,

    stale_belief_actions: u64,
    contradicted_belief_actions: u64,
    source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>,
    route_preference_changes: u64,
    correction_latency_values: Vec<u64>,
    blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>,

    queue_wait_values: Vec<u64>,
    reservation_conflict_count: u64,
    abandoned_grant_count: u64,
    dead_claimant_cleanup_count: u64,

    opportunity_compiled_values: Vec<u64>,
    opportunity_salience_floored_values: Vec<u64>,
    opportunity_learned_memory_damped_values: Vec<u64>,
    opportunity_cap_truncated_values: Vec<u64>,
    search_expansion_values: Vec<u64>,
    cache_hit_count: u64,
    cache_miss_count: u64,
    cache_invalidation_count: u64,
    planning_state_cache_entities_at_hits: u64,
    planning_state_cache_entities_at_misses: u64,
    planning_state_cache_effective_place_hits: u64,
    planning_state_cache_effective_place_misses: u64,
    planning_state_cache_invalidations: u64,
}

impl ScenarioDiagnosticsBuilder {
    fn record_decision_trace(&mut self, trace: &AgentDecisionTrace) {
        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            return;
        };

        self.planning_decision_count += 1;
        if planning.plan_continued {
            self.active_intention_continuation_count += 1;
        }

        let planned_opportunities: BTreeSet<_> = planning
            .planning
            .attempts
            .iter()
            .map(|attempt| worldwake_core::OpportunityKey {
                goal_key: attempt.goal,
                anchor: attempt.opportunity_anchor,
            })
            .collect();

        for opportunity in &planning.candidates.generated {
            increment(
                &mut self.candidates_emitted_by_kind,
                opportunity.goal_key.kind,
                1,
            );
        }
        if let Some(portfolio) = &planning.portfolio {
            for slot in portfolio.slots.keys().copied() {
                increment(&mut self.candidates_emitted_by_slot, slot, 1);
            }
        }
        for ranked in &planning.candidates.ranked {
            if !planned_opportunities.contains(&ranked.opportunity) {
                increment(
                    &mut self.top_k_not_planned,
                    ranked.opportunity.goal_key.kind,
                    1,
                );
            }
        }

        self.add_suppression(
            CandidateSuppressionCategory::SituationallySuppressed,
            planning.candidates.suppressed.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::SoftDamped,
            planning.candidates.damped.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::ZeroMotive,
            planning.candidates.zero_motive.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::FullyBlockedDesire,
            planning.candidates.fully_blocked_desires.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::OmittedPolitical,
            planning.candidates.omitted_political.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::OmittedBandit,
            planning.candidates.omitted_bandit.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::OmittedSocial,
            planning.candidates.omitted_social.len(),
        );
        self.add_suppression(
            CandidateSuppressionCategory::OmittedViolationDetection,
            planning.candidates.omitted_violation_detection.len(),
        );

        if let Some(load) = trace.opportunity_compiler_load {
            self.opportunity_compiled_values
                .push(u64::from(load.compiled_count));
            self.opportunity_salience_floored_values
                .push(u64::from(load.salience_floored));
            self.opportunity_learned_memory_damped_values
                .push(u64::from(load.learned_memory_damped));
            self.opportunity_cap_truncated_values
                .push(u64::from(load.cap_truncated));
        }
        if let Some(counters) = trace.snapshot_cache_counters {
            self.cache_hit_count = self
                .cache_hit_count
                .saturating_add(counters.cache_hit_count);
            self.cache_miss_count = self
                .cache_miss_count
                .saturating_add(counters.cache_miss_count);
            self.cache_invalidation_count = self
                .cache_invalidation_count
                .saturating_add(counters.cache_invalidation_count);
        }
        if let Some(counters) = trace.planning_state_cache_counters {
            self.planning_state_cache_entities_at_hits = self
                .planning_state_cache_entities_at_hits
                .saturating_add(counters.entities_at_hits);
            self.planning_state_cache_entities_at_misses = self
                .planning_state_cache_entities_at_misses
                .saturating_add(counters.entities_at_misses);
            self.planning_state_cache_effective_place_hits = self
                .planning_state_cache_effective_place_hits
                .saturating_add(counters.effective_place_hits);
            self.planning_state_cache_effective_place_misses = self
                .planning_state_cache_effective_place_misses
                .saturating_add(counters.effective_place_misses);
            self.planning_state_cache_invalidations = self
                .planning_state_cache_invalidations
                .saturating_add(counters.invalidations);
        }
    }

    fn record_plan_trace(&mut self, trace: &PlanAttemptTrace) {
        self.plan_attempts += 1;
        increment(&mut self.plan_attempts_by_kind, trace.goal.kind, 1);
        self.record_method_usage(trace);

        match &trace.outcome {
            PlanSearchOutcome::Found {
                steps,
                terminal_kind,
            } => {
                self.plan_depth_values.push(steps.len() as u64);
                increment(
                    &mut self.terminal_kind_distribution,
                    crate::PlanTerminalKindDiscriminant::from(terminal_kind),
                    1,
                );
            }
            PlanSearchOutcome::BudgetExhausted { .. } => {
                self.budget_exhaustion_count += 1;
            }
            PlanSearchOutcome::FrontierExhausted { .. } => {
                self.frontier_exhaustion_count += 1;
            }
            PlanSearchOutcome::Unsupported => {}
        }

        for expansion in &trace.expansion_summaries {
            self.search_expansion_values
                .push(u64::from(expansion.candidates_generated));
            self.beam_candidate_count = self
                .beam_candidate_count
                .saturating_add(u64::from(expansion.non_terminal_before_beam));
            self.beam_truncated_count = self.beam_truncated_count.saturating_add(u64::from(
                expansion
                    .non_terminal_before_beam
                    .saturating_sub(expansion.non_terminal_after_beam),
            ));
            self.heuristic_expansions += 1;
            if expansion.helpful_action_count > 0 {
                self.heuristic_helpful_expansions += 1;
            }
        }
    }

    fn record_method_usage(&mut self, trace: &PlanAttemptTrace) {
        let key = trace
            .method_trace
            .as_ref()
            .and_then(|method_trace| method_trace.method_id);
        let counts = self.method_usage.entry(key).or_default();
        counts.attempts = counts.attempts.saturating_add(1);
        if let Some(method_trace) = &trace.method_trace {
            if method_trace.method_id.is_some() {
                counts.selected_count = counts.selected_count.saturating_add(1);
            } else {
                counts.fallback_count = counts.fallback_count.saturating_add(1);
            }
            if method_trace.failure_mode.is_some() {
                counts.failure_count = counts.failure_count.saturating_add(1);
            }
        } else {
            counts.fallback_count = counts.fallback_count.saturating_add(1);
        }
    }

    fn record_repair_trace(&mut self, trace: &RepairAttemptTrace) {
        self.repair_attempts += 1;
        if trace.chosen_kind.is_some() {
            self.repair_succeeded += 1;
        }
        self.repair_budget_values
            .push(u64::from(trace.budget_consumed));
        if trace.chosen_kind.is_none() && trace.budget_consumed >= trace.budget_total {
            self.full_replan_count += 1;
        }
    }

    fn record_event_log(&mut self, event_log: &EventLog, tick_range: (Tick, Tick)) {
        for tick in ticks_in_range(tick_range) {
            for event_id in event_log.events_at_tick(tick) {
                let Some(record) = event_log.get(*event_id) else {
                    continue;
                };

                if let Some(payload) = record.decision_payload() {
                    self.record_decision_payload(payload);
                }
            }
        }

        self.record_contention_events(event_log, EventTag::ContentionResolved, tick_range);
        self.record_contention_events(event_log, EventTag::QueueGrantPromoted, tick_range);
        self.abandoned_grant_count =
            count_tagged_events_in_range(event_log, EventTag::QueueGrantExpired, tick_range);
        self.dead_claimant_cleanup_count =
            count_tagged_events_in_range(event_log, EventTag::QueueHeadFailed, tick_range);
    }

    fn record_decision_payload(&mut self, payload: &DecisionEventPayload) {
        match payload {
            DecisionEventPayload::GoalSuppressed(payload) => {
                self.add_suppression(goal_rejection_category(payload.reason), 1);
                self.record_testimony_trust_context(&payload.testimony_trust_context);
            }
            DecisionEventPayload::GoalCommitted(payload) => {
                self.record_testimony_trust_context(&payload.testimony_trust_context);
                self.route_preference_changes = self.route_preference_changes.saturating_add(
                    u64::try_from(payload.route_preference_context.len()).unwrap_or(u64::MAX),
                );
            }
            DecisionEventPayload::PlanInvalidated(payload) => {
                self.record_plan_invalidation_reason(&payload.reason);
                self.record_belief_snapshot(
                    payload.belief_snapshot.map(|snapshot| snapshot.status),
                );
            }
            DecisionEventPayload::ReplanTriggered(payload) => {
                self.record_replan_reason(&payload.reason);
            }
            DecisionEventPayload::BlockerRecorded(payload) => {
                increment(
                    &mut self.blocker_counts_by_scope,
                    blocker_scope_variant(payload.scope),
                    1,
                );
                if let Some(discrepancy) = payload.discrepancy {
                    self.record_discrepancy(discrepancy);
                }
                self.record_belief_snapshot(
                    payload.belief_snapshot.map(|snapshot| snapshot.status),
                );
            }
            _ => {}
        }
    }

    fn record_testimony_trust_context(
        &mut self,
        testimony_trust_context: &[worldwake_core::TestimonyTrustSummary],
    ) {
        for summary in testimony_trust_context {
            increment(
                &mut self.source_reliability_changes_by_topic,
                summary.topic,
                1,
            );
        }
    }

    fn record_plan_invalidation_reason(&mut self, reason: &PlanInvalidationReason) {
        if let PlanInvalidationReason::DiscrepancyRecorded { discrepancy } = reason {
            self.record_discrepancy(*discrepancy);
        }
    }

    fn record_replan_reason(&mut self, reason: &ReplanReason) {
        match reason {
            ReplanReason::PlanInvalidated { reason } => {
                self.record_plan_invalidation_reason(reason);
            }
            ReplanReason::DiscrepancyRecorded { discrepancy } => {
                self.record_discrepancy(*discrepancy);
            }
            ReplanReason::LocalRepairExhausted | ReplanReason::SearchBudgetExhausted => {
                self.full_replan_count += 1;
            }
            _ => {}
        }
    }

    fn record_discrepancy(&mut self, discrepancy: Discrepancy) {
        let normalized = normalize_discrepancy(discrepancy);
        increment(&mut self.invalidation_reasons, normalized, 1);
        match normalized {
            Discrepancy::BeliefStale => self.stale_belief_actions += 1,
            Discrepancy::BeliefContradicted => self.contradicted_belief_actions += 1,
            _ => {}
        }
    }

    fn record_belief_snapshot(&mut self, status: Option<BeliefStatusTag>) {
        match status {
            Some(BeliefStatusTag::Stale) => self.stale_belief_actions += 1,
            Some(BeliefStatusTag::Contradicted) => self.contradicted_belief_actions += 1,
            _ => {}
        }
    }

    fn record_contention_events(
        &mut self,
        event_log: &EventLog,
        tag: EventTag,
        tick_range: (Tick, Tick),
    ) {
        for event_id in event_log.events_by_tag(tag) {
            let Some(record) = event_log.get(*event_id) else {
                continue;
            };
            if !tick_in_range(record.tick(), tick_range) {
                continue;
            }
            let Some(payload) = record.contention_event_payload() else {
                continue;
            };

            if payload.total_claimants > 1 {
                self.reservation_conflict_count += 1;
            }

            for claimant in &payload.claimants {
                if claimant.outcome == ClaimantOutcome::Granted {
                    self.queue_wait_values
                        .push(payload.at_tick.0.saturating_sub(claimant.arrived_tick.0));
                }
            }
        }
    }

    fn add_suppression(&mut self, category: CandidateSuppressionCategory, count: usize) {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        if count == 0 {
            return;
        }
        increment(&mut self.candidates_suppressed_by_category, category, count);
    }

    fn finish(mut self, tick_range: (Tick, Tick)) -> ScenarioDiagnosticsReport {
        ScenarioDiagnosticsReport {
            tick_range,
            goal_pressure: GoalPressureMetrics {
                candidates_emitted_by_kind: self.candidates_emitted_by_kind,
                candidates_emitted_by_slot: self.candidates_emitted_by_slot,
                candidates_suppressed_by_category: self.candidates_suppressed_by_category,
                top_k_not_planned: self.top_k_not_planned,
                active_intention_continuation_rate: ratio_permille(
                    self.active_intention_continuation_count,
                    self.planning_decision_count,
                ),
            },
            planning: PlanningMetrics {
                plan_attempts: self.plan_attempts,
                plan_attempts_by_kind: self.plan_attempts_by_kind,
                budget_exhaustion_count: self.budget_exhaustion_count,
                budget_exhaustion_rate: ratio_permille(
                    self.budget_exhaustion_count,
                    self.plan_attempts,
                ),
                frontier_exhaustion_count: self.frontier_exhaustion_count,
                frontier_exhaustion_rate: ratio_permille(
                    self.frontier_exhaustion_count,
                    self.plan_attempts,
                ),
                beam_truncation_ratio: ratio_permille(
                    self.beam_truncated_count,
                    self.beam_candidate_count,
                ),
                plan_depth: bucket_from_unsorted(&mut self.plan_depth_values),
                terminal_kind_distribution: self.terminal_kind_distribution,
                heuristic_helpful_action_hit_rate: ratio_permille(
                    self.heuristic_helpful_expansions,
                    self.heuristic_expansions,
                ),
                method_usage: self.method_usage,
            },
            revalidation_repair: RevalidationRepairMetrics {
                invalidation_reasons: self.invalidation_reasons,
                repair_attempts: self.repair_attempts,
                repair_succeeded: self.repair_succeeded,
                repair_failed: self.repair_attempts.saturating_sub(self.repair_succeeded),
                repair_success_rate: ratio_permille(self.repair_succeeded, self.repair_attempts),
                repair_budget_consumed: bucket_from_unsorted(&mut self.repair_budget_values),
                full_replan_count: self.full_replan_count,
            },
            belief: BeliefMetrics {
                stale_belief_actions: self.stale_belief_actions,
                contradicted_belief_actions: self.contradicted_belief_actions,
                source_reliability_changes_by_topic: self.source_reliability_changes_by_topic,
                route_preference_changes: self.route_preference_changes,
                false_rumor_propagation_count: 0,
                correction_latency: bucket_from_unsorted(&mut self.correction_latency_values),
                blocker_counts_by_scope: self.blocker_counts_by_scope,
            },
            coordination: CoordinationMetrics {
                queue_wait_ticks: bucket_from_unsorted(&mut self.queue_wait_values),
                reservation_conflict_count: self.reservation_conflict_count,
                abandoned_grant_count: self.abandoned_grant_count,
                dead_claimant_cleanup_count: self.dead_claimant_cleanup_count,
            },
            performance: PerformanceMetrics {
                opportunity_compiled_count: bucket_from_unsorted(
                    &mut self.opportunity_compiled_values,
                ),
                opportunity_salience_floored: bucket_from_unsorted(
                    &mut self.opportunity_salience_floored_values,
                ),
                opportunity_learned_memory_damped: bucket_from_unsorted(
                    &mut self.opportunity_learned_memory_damped_values,
                ),
                opportunity_cap_truncated: bucket_from_unsorted(
                    &mut self.opportunity_cap_truncated_values,
                ),
                search_expansions: bucket_from_unsorted(&mut self.search_expansion_values),
                cache_hit_count: self.cache_hit_count,
                cache_miss_count: self.cache_miss_count,
                cache_invalidation_count: self.cache_invalidation_count,
                planning_state_cache_entities_at_hits: self.planning_state_cache_entities_at_hits,
                planning_state_cache_entities_at_misses: self
                    .planning_state_cache_entities_at_misses,
                planning_state_cache_effective_place_hits: self
                    .planning_state_cache_effective_place_hits,
                planning_state_cache_effective_place_misses: self
                    .planning_state_cache_effective_place_misses,
                planning_state_cache_invalidations: self.planning_state_cache_invalidations,
            },
        }
    }
}

fn blocker_scope_variant(scope: BlockerScope) -> BlockerScopeVariantId {
    match scope {
        BlockerScope::Exact(_) => BlockerScopeVariantId::Exact,
        BlockerScope::RouteSegment(_) => BlockerScopeVariantId::RouteSegment,
        BlockerScope::Counterparty(_) => BlockerScopeVariantId::Counterparty,
    }
}

fn increment<K: Ord>(map: &mut BTreeMap<K, u64>, key: K, count: u64) {
    map.entry(key)
        .and_modify(|current| *current = current.saturating_add(count))
        .or_insert(count);
}

fn bucket_from_unsorted(values: &mut [u64]) -> PercentileBucket {
    values.sort_unstable();
    PercentileBucket::from_sorted(values)
}

fn ratio_permille(numerator: u64, denominator: u64) -> Permille {
    if denominator == 0 {
        return Permille::ZERO;
    }

    let value = numerator.saturating_mul(1000) / denominator;
    Permille::new(u16::try_from(value.min(1000)).expect("ratio is clamped to permille bounds"))
        .expect("ratio is clamped to permille bounds")
}

fn ticks_in_range(tick_range: (Tick, Tick)) -> impl Iterator<Item = Tick> {
    let start = tick_range.0.0;
    let end = tick_range.1.0;
    (start..=end)
        .take(if start <= end { usize::MAX } else { 0 })
        .map(Tick)
}

fn tick_in_range(tick: Tick, tick_range: (Tick, Tick)) -> bool {
    tick_range.0 <= tick && tick <= tick_range.1
}

fn count_tagged_events_in_range(
    event_log: &EventLog,
    tag: EventTag,
    tick_range: (Tick, Tick),
) -> u64 {
    event_log
        .events_by_tag(tag)
        .iter()
        .filter_map(|event_id| event_log.get(*event_id))
        .filter(|record| tick_in_range(record.tick(), tick_range))
        .count()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn goal_rejection_category(reason: GoalRejectionReason) -> CandidateSuppressionCategory {
    match reason {
        GoalRejectionReason::LowerMotive => CandidateSuppressionCategory::RejectedLowerMotive,
        GoalRejectionReason::FeasibilityProbeFailed => {
            CandidateSuppressionCategory::RejectedFeasibilityProbeFailed
        }
        GoalRejectionReason::SuppressedByBlocker => {
            CandidateSuppressionCategory::RejectedSuppressedByBlocker
        }
        GoalRejectionReason::SuppressedByDiscrepancy
        | GoalRejectionReason::SuppressedByUnreliableTestimony => {
            CandidateSuppressionCategory::RejectedSuppressedByDiscrepancy
        }
        GoalRejectionReason::SuppressedByStressPolicy => {
            CandidateSuppressionCategory::RejectedSuppressedByStressPolicy
        }
        GoalRejectionReason::SuppressedByContentionPreempt => {
            CandidateSuppressionCategory::RejectedSuppressedByContentionPreempt
        }
        GoalRejectionReason::ArbitrationLost => {
            CandidateSuppressionCategory::RejectedArbitrationLost
        }
        GoalRejectionReason::SwitchMarginInsufficient => {
            CandidateSuppressionCategory::RejectedSwitchMarginInsufficient
        }
    }
}

fn normalize_discrepancy(discrepancy: Discrepancy) -> Discrepancy {
    match discrepancy {
        Discrepancy::NeedHorizonExceeded { .. } => Discrepancy::NeedHorizonExceeded {
            need: worldwake_core::HomeostaticNeedId::Hunger,
            projected_breach_tick: Tick(0),
        },
        Discrepancy::Omission(_) => Discrepancy::Omission(OmissionReason::OverBudget {
            budget: 0,
            candidates_seen: 0,
        }),
        Discrepancy::ArtifactNotActionable { .. } => Discrepancy::ArtifactNotActionable {
            artifact: worldwake_core::EntityId {
                slot: 0,
                generation: 0,
            },
            reason: worldwake_core::BlockerReason::LegalEffectExpired,
        },
        Discrepancy::MethodFailure(_) => {
            Discrepancy::MethodFailure(worldwake_core::MethodFailureContext {
                method_id: worldwake_core::MethodSchemaId(0),
                kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                subgoal_index: None,
            })
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{MethodUsageCounts, build_scenario_diagnostics};
    use crate::agent_tick::portfolio::FeasibilityVerdict;
    use crate::decision_trace::{
        MethodPlanAttemptTrace, OpportunityCompilerLoad, PortfolioSlotTrace, PortfolioTrace,
        SearchExpansionSummary, TargetBeliefPresence,
    };
    use crate::htn::MethodFailureMode;
    use crate::{
        BlockerScopeVariantId, CandidateDampingEntry, CandidateSuppressionCategory, CandidateTrace,
        DecisionOutcome, ExecutionTrace, GoalPriorityClass, PlanAttemptTrace, PlanSearchOutcome,
        PlanSearchTrace, PlannedStepSummary, PlanningPipelineTrace, RankedGoalSummary,
        RepairAttemptTrace, RepairFailure, SelectionTrace, SnapshotCacheCounters,
    };
    use worldwake_core::{
        ActionDefId, BeliefStatusTag, BlockerKey, BlockerRecordedPayload, BlockerScope,
        BlockingFact, CauseRef, ClaimantOutcome, CommodityKind, ContentionClaimant,
        ContentionEventPayload, ContentionResolutionRule, DecisionEventPayload, Discrepancy,
        EntityId, EventLog, EventPayload, EventTag, GoalCommittedPayload, GoalKey, GoalKind,
        GoalRejectionReason, GoalSuppressedPayload, HomeostaticNeedId, InvalidatorTag,
        MethodSchemaId, OmissionReason, OpportunityAnchor, OpportunityKey, PendingEvent,
        PercentileBucket, Permille, PlanInvalidatedPayload, PlanInvalidationReason, RepairKind,
        ReplanReason, ReplanTriggeredPayload, RoutePreferenceSummary, RouteSegment,
        TestimonyTrustSummary, Tick, TopicScope, VisibilitySpec, WitnessData,
    };

    #[test]
    fn build_scenario_diagnostics_is_deterministic_for_identical_inputs() {
        let decision_traces = vec![planning_decision_trace()];
        let plan_traces = vec![found_plan_attempt(GoalKind::Sleep, 2)];
        let repair_traces = vec![repair_trace(Some(RepairKind::InsertVerification), 3, 5)];
        let event_log = EventLog::new();

        let first = build_scenario_diagnostics(
            &decision_traces,
            &plan_traces,
            &repair_traces,
            &event_log,
            (Tick(0), Tick(3)),
        );
        let second = build_scenario_diagnostics(
            &decision_traces,
            &plan_traces,
            &repair_traces,
            &event_log,
            (Tick(0), Tick(3)),
        );

        assert_eq!(first, second);
        assert_eq!(
            bincode::serialize(&first).unwrap(),
            bincode::serialize(&second).unwrap()
        );
    }

    #[test]
    fn goal_pressure_metrics_roll_up_candidate_and_suppression_sources() {
        let mut event_log = EventLog::new();
        emit_decision_payload(
            &mut event_log,
            Tick(1),
            EventTag::GoalSuppressed,
            DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                reason: GoalRejectionReason::FeasibilityProbeFailed,
                testimony_trust_context: Vec::new(),
            }),
        );

        let report = build_scenario_diagnostics(
            &[planning_decision_trace()],
            &[],
            &[],
            &event_log,
            (Tick(0), Tick(3)),
        );

        assert_eq!(
            report.goal_pressure.candidates_emitted_by_kind,
            BTreeMap::from([(GoalKind::Sleep, 1), (wash_goal(), 1)])
        );
        assert_eq!(
            report.goal_pressure.candidates_emitted_by_slot,
            BTreeMap::from([
                (crate::SlotKind::NeedSurvival, 1),
                (crate::SlotKind::EconomicOpportunity, 1)
            ])
        );
        assert_eq!(
            report.goal_pressure.candidates_suppressed_by_category,
            BTreeMap::from([
                (
                    CandidateSuppressionCategory::RejectedFeasibilityProbeFailed,
                    1
                ),
                (CandidateSuppressionCategory::SituationallySuppressed, 1),
                (CandidateSuppressionCategory::SoftDamped, 1),
                (CandidateSuppressionCategory::ZeroMotive, 1),
                (CandidateSuppressionCategory::FullyBlockedDesire, 1),
                (CandidateSuppressionCategory::OmittedViolationDetection, 1),
            ])
        );
        assert_eq!(
            report.goal_pressure.top_k_not_planned,
            BTreeMap::from([(wash_goal(), 1)])
        );
        assert_eq!(
            report.goal_pressure.active_intention_continuation_rate,
            Permille::new_unchecked(1000)
        );
    }

    #[test]
    fn belief_metrics_roll_up_testimony_topics_and_route_preferences() {
        let mut event_log = EventLog::new();
        emit_decision_payload(
            &mut event_log,
            Tick(1),
            EventTag::GoalCommitted,
            DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                motive_score: 100,
                decisive_motive_sources: Vec::new(),
                rejected_alternatives: Vec::new(),
                assumptions: Vec::new(),
                testimony_trust_context: vec![
                    testimony_summary(TopicScope::RouteHazard),
                    testimony_summary(TopicScope::ResourceAvailability),
                ],
                route_preference_context: vec![RoutePreferenceSummary {
                    segment: RouteSegment::new(entity(10), entity(11)),
                    preference: Permille::new_unchecked(700),
                    last_safe_tick: Some(Tick(1)),
                    last_dangerous_tick: None,
                }],
            }),
        );
        emit_decision_payload(
            &mut event_log,
            Tick(2),
            EventTag::GoalSuppressed,
            DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                agent: entity(1),
                goal_key: GoalKey::from(wash_goal()),
                reason: GoalRejectionReason::SuppressedByUnreliableTestimony,
                testimony_trust_context: vec![testimony_summary(TopicScope::RouteHazard)],
            }),
        );

        let report = build_scenario_diagnostics(&[], &[], &[], &event_log, (Tick(0), Tick(3)));

        assert_eq!(
            report.belief.source_reliability_changes_by_topic,
            BTreeMap::from([
                (TopicScope::RouteHazard, 2),
                (TopicScope::ResourceAvailability, 1),
            ])
        );
        assert_eq!(report.belief.route_preference_changes, 1);
    }

    #[test]
    fn planning_metrics_roll_up_outcomes_depths_and_expansion_counts() {
        let plan_traces = vec![
            found_plan_attempt(GoalKind::Sleep, 3),
            PlanAttemptTrace {
                goal: GoalKey::from(wash_goal()),
                opportunity_anchor: OpportunityAnchor::None,
                outcome: PlanSearchOutcome::BudgetExhausted { expansions_used: 4 },
                goal_budget: worldwake_core::GoalPlanningBudget::TRAVEL_PURCHASE,
                strategic_budget: None,
                strategic_plan: None,
                tactical_goal: None,
                landmarks_extracted: 0,
                landmark_orderings: 0,
                target_belief_presence: TargetBeliefPresence::NotApplicable,
                method_trace: None,
                binding_rejections: Vec::new(),
                expansion_summaries: vec![SearchExpansionSummary {
                    depth: 1,
                    remaining_travel_ticks: 0,
                    combined_places_count: 0,
                    prerequisite_places_count: 0,
                    candidates_generated: 7,
                    candidates_skipped: 0,
                    terminal_successors: 0,
                    non_terminal_before_beam: 8,
                    non_terminal_after_beam: 3,
                    found_goal_satisfied: false,
                    preferred_candidates: 0,
                    landmark_heuristic: 0,
                    ff_heuristic: Some(2),
                    helpful_action_count: 1,
                    travel_pruning: None,
                    prerequisite_guidance: None,
                    expansion_candidates: Vec::new(),
                    root_candidates: Vec::new(),
                    root_omissions: Vec::new(),
                }],
            },
            PlanAttemptTrace {
                goal: GoalKey::from(wash_goal()),
                opportunity_anchor: OpportunityAnchor::None,
                outcome: PlanSearchOutcome::FrontierExhausted { expansions_used: 2 },
                goal_budget: worldwake_core::GoalPlanningBudget::TRAVEL_PURCHASE,
                strategic_budget: None,
                strategic_plan: None,
                tactical_goal: None,
                landmarks_extracted: 0,
                landmark_orderings: 0,
                target_belief_presence: TargetBeliefPresence::NotApplicable,
                method_trace: None,
                binding_rejections: Vec::new(),
                expansion_summaries: Vec::new(),
            },
        ];

        let report = build_scenario_diagnostics(
            &[],
            &plan_traces,
            &[],
            &EventLog::new(),
            (Tick(0), Tick(0)),
        );

        assert_eq!(report.planning.plan_attempts, 3);
        assert_eq!(report.planning.budget_exhaustion_count, 1);
        assert_eq!(
            report.planning.budget_exhaustion_rate,
            Permille::new_unchecked(333)
        );
        assert_eq!(report.planning.frontier_exhaustion_count, 1);
        assert_eq!(
            report.planning.frontier_exhaustion_rate,
            Permille::new_unchecked(333)
        );
        assert_eq!(
            report.planning.beam_truncation_ratio,
            Permille::new_unchecked(625)
        );
        assert_eq!(
            report.planning.plan_depth,
            PercentileBucket::from_sorted(&[3])
        );
        assert_eq!(
            report.planning.terminal_kind_distribution,
            BTreeMap::from([(crate::PlanTerminalKindDiscriminant::GoalSatisfied, 1)])
        );
        assert_eq!(
            report.planning.heuristic_helpful_action_hit_rate,
            Permille::new_unchecked(1000)
        );
        assert_eq!(
            report.planning.method_usage.get(&None),
            Some(&MethodUsageCounts {
                attempts: 3,
                selected_count: 0,
                fallback_count: 3,
                failure_count: 0,
            })
        );
        assert_eq!(
            report.performance.search_expansions,
            PercentileBucket::from_sorted(&[7])
        );
    }

    #[test]
    fn planning_metrics_aggregate_terminal_discriminants_without_payload_fragmentation() {
        let mut first = found_plan_attempt(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: worldwake_core::AcquisitionQuantity::single(),
            },
            1,
        );
        let mut second = found_plan_attempt(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Grain,
                purpose: worldwake_core::CommodityPurpose::Restock,
                quantity: worldwake_core::AcquisitionQuantity::single(),
            },
            2,
        );

        if let PlanSearchOutcome::Found { terminal_kind, .. } = &mut first.outcome {
            *terminal_kind = crate::PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Bread,
                place: entity(10),
            };
        }
        if let PlanSearchOutcome::Found { terminal_kind, .. } = &mut second.outcome {
            *terminal_kind = crate::PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Grain,
                place: entity(11),
            };
        }

        let report = build_scenario_diagnostics(
            &[],
            &[first, second],
            &[],
            &EventLog::new(),
            (Tick(0), Tick(0)),
        );

        assert_eq!(
            report.planning.terminal_kind_distribution,
            BTreeMap::from([(crate::PlanTerminalKindDiscriminant::ResourceBarrier, 2)])
        );
    }

    #[test]
    fn method_usage_counts_selected_fallback_and_failure_correctly() {
        let mut selected = found_plan_attempt(GoalKind::Sleep, 1);
        selected.method_trace = Some(MethodPlanAttemptTrace {
            method_id: Some(MethodSchemaId(5)),
            subgoals_attempted: Vec::new(),
            failure_mode: None,
            motive_score: 400,
        });
        let mut failed = found_plan_attempt(GoalKind::Sleep, 1);
        failed.method_trace = Some(MethodPlanAttemptTrace {
            method_id: Some(MethodSchemaId(5)),
            subgoals_attempted: Vec::new(),
            failure_mode: Some(MethodFailureMode::Timeout(12)),
            motive_score: 400,
        });
        let fallback = found_plan_attempt(GoalKind::Wash, 1);

        let report = build_scenario_diagnostics(
            &[],
            &[selected, failed, fallback],
            &[],
            &EventLog::new(),
            (Tick(0), Tick(0)),
        );

        assert_eq!(
            report.planning.method_usage.get(&Some(MethodSchemaId(5))),
            Some(&MethodUsageCounts {
                attempts: 2,
                selected_count: 2,
                fallback_count: 0,
                failure_count: 1,
            })
        );
        assert_eq!(
            report.planning.method_usage.get(&None),
            Some(&MethodUsageCounts {
                attempts: 1,
                selected_count: 0,
                fallback_count: 1,
                failure_count: 0,
            })
        );
    }

    #[test]
    fn revalidation_belief_and_coordination_metrics_roll_up_event_payloads() {
        let mut event_log = EventLog::new();
        emit_decision_payload(
            &mut event_log,
            Tick(1),
            EventTag::PlanInvalidated,
            DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                reason: PlanInvalidationReason::DiscrepancyRecorded {
                    discrepancy: Discrepancy::NeedHorizonExceeded {
                        need: HomeostaticNeedId::Hunger,
                        projected_breach_tick: Tick(5),
                    },
                },
                belief_snapshot: Some(worldwake_core::BeliefSnapshot {
                    confidence: Permille::new_unchecked(200),
                    status: BeliefStatusTag::Stale,
                    acquired_tick: Tick(0),
                }),
            }),
        );
        emit_decision_payload(
            &mut event_log,
            Tick(2),
            EventTag::ReplanTriggered,
            DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                reason: ReplanReason::DiscrepancyRecorded {
                    discrepancy: Discrepancy::NeedHorizonExceeded {
                        need: HomeostaticNeedId::Thirst,
                        projected_breach_tick: Tick(9),
                    },
                },
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
        );
        emit_contention_payload(&mut event_log, Tick(3), EventTag::ContentionResolved);
        emit_contention_payload(&mut event_log, Tick(4), EventTag::QueueGrantPromoted);
        emit_plain_event(&mut event_log, Tick(5), EventTag::QueueGrantExpired);
        emit_plain_event(&mut event_log, Tick(6), EventTag::QueueHeadFailed);

        let repair_traces = vec![
            repair_trace(Some(RepairKind::RebindTarget), 2, 5),
            repair_trace(None, 5, 5),
        ];

        let report =
            build_scenario_diagnostics(&[], &[], &repair_traces, &event_log, (Tick(0), Tick(10)));

        let normalized_need_horizon = Discrepancy::NeedHorizonExceeded {
            need: HomeostaticNeedId::Hunger,
            projected_breach_tick: Tick(0),
        };
        assert_eq!(
            report.revalidation_repair.invalidation_reasons,
            BTreeMap::from([(normalized_need_horizon, 2)])
        );
        assert_eq!(report.revalidation_repair.repair_attempts, 2);
        assert_eq!(report.revalidation_repair.repair_succeeded, 1);
        assert_eq!(report.revalidation_repair.repair_failed, 1);
        assert_eq!(
            report.revalidation_repair.repair_success_rate,
            Permille::new_unchecked(500)
        );
        assert_eq!(report.revalidation_repair.full_replan_count, 1);
        assert_eq!(report.belief.stale_belief_actions, 1);
        assert_eq!(
            report.coordination.queue_wait_ticks,
            PercentileBucket::from_sorted(&[2, 2])
        );
        assert_eq!(report.coordination.reservation_conflict_count, 2);
        assert_eq!(report.coordination.abandoned_grant_count, 1);
        assert_eq!(report.coordination.dead_claimant_cleanup_count, 1);
    }

    #[test]
    fn aggregator_populates_blocker_counts_by_scope() {
        let mut event_log = EventLog::new();
        for tick in [1, 2, 3] {
            emit_blocker_recorded(
                &mut event_log,
                Tick(tick),
                BlockerScope::Exact(BlockerKey {
                    goal_key: GoalKey::from(GoalKind::Sleep),
                    place: Some(entity(10)),
                    target: None,
                    action_def: None,
                }),
            );
        }
        for tick in [4, 5] {
            emit_blocker_recorded(
                &mut event_log,
                Tick(tick),
                BlockerScope::RouteSegment(RouteSegment::new(entity(10), entity(11))),
            );
        }
        emit_blocker_recorded(
            &mut event_log,
            Tick(6),
            BlockerScope::Counterparty(entity(20)),
        );

        let report = build_scenario_diagnostics(&[], &[], &[], &event_log, (Tick(0), Tick(10)));

        assert_eq!(
            report.belief.blocker_counts_by_scope,
            BTreeMap::from([
                (BlockerScopeVariantId::Exact, 3),
                (BlockerScopeVariantId::RouteSegment, 2),
                (BlockerScopeVariantId::Counterparty, 1),
            ])
        );
        assert_eq!(
            report.belief.blocker_counts_by_scope.values().sum::<u64>(),
            6
        );
    }

    #[test]
    fn performance_metrics_roll_up_opportunity_load_and_cache_counters() {
        let report = build_scenario_diagnostics(
            &[planning_decision_trace()],
            &[],
            &[],
            &EventLog::new(),
            (Tick(0), Tick(0)),
        );

        assert_eq!(
            report.performance.opportunity_compiled_count,
            PercentileBucket::from_sorted(&[5])
        );
        assert_eq!(
            report.performance.opportunity_salience_floored,
            PercentileBucket::from_sorted(&[1])
        );
        assert_eq!(
            report.performance.opportunity_learned_memory_damped,
            PercentileBucket::from_sorted(&[2])
        );
        assert_eq!(
            report.performance.opportunity_cap_truncated,
            PercentileBucket::from_sorted(&[3])
        );
        assert_eq!(report.performance.cache_hit_count, 11);
        assert_eq!(report.performance.cache_miss_count, 4);
        assert_eq!(report.performance.cache_invalidation_count, 0);
        assert_eq!(report.performance.planning_state_cache_entities_at_hits, 3);
        assert_eq!(
            report.performance.planning_state_cache_entities_at_misses,
            2
        );
        assert_eq!(
            report.performance.planning_state_cache_effective_place_hits,
            5
        );
        assert_eq!(
            report
                .performance
                .planning_state_cache_effective_place_misses,
            4
        );
        assert_eq!(report.performance.planning_state_cache_invalidations, 1);
    }

    fn planning_decision_trace() -> crate::AgentDecisionTrace {
        let sleep = opportunity(GoalKind::Sleep);
        let wash = opportunity(wash_goal());
        crate::AgentDecisionTrace {
            agent: entity(1),
            tick: Tick(1),
            outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
                affordances: None,
                dirty: crate::DirtySet::NO_PLAN,
                plan_continued: true,
                candidates: CandidateTrace {
                    generated: vec![sleep, wash],
                    evidence: Vec::new(),
                    fully_blocked_desires: vec![crate::DesireFullyBlocked {
                        goal_key: GoalKey::from(wash_goal()),
                        blocked_opportunities: Vec::new(),
                        blocker_matches: Vec::new(),
                    }],
                    places_reachable: 0,
                    places_after_belief_filter: 0,
                    ranked: vec![ranked(sleep), ranked(wash)],
                    top_ranked_comparison: None,
                    suppressed: vec![GoalKey::from(wash_goal())],
                    damped: vec![CandidateDampingEntry {
                        goal_key: GoalKey::from(GoalKind::Sleep),
                        reason: crate::CandidateDampingReason::SurveyMemoryNegative {
                            place: entity(9),
                            hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                                commodity: CommodityKind::Bread,
                            },
                            recorded_tick: Tick(0),
                            confidence: Permille::new_unchecked(250),
                        },
                    }],
                    zero_motive: vec![GoalKey::from(wash_goal())],
                    omitted_political: Vec::new(),
                    omitted_bandit: Vec::new(),
                    omitted_social: Vec::new(),
                    omitted_testimony: Vec::new(),
                    omitted_violation_detection: vec![crate::ViolationDetectionOmission {
                        reason: crate::ViolationDetectionOmissionReason::AgentInTransit,
                    }],
                },
                planning: PlanSearchTrace {
                    attempts: vec![found_plan_attempt(GoalKind::Sleep, 1)],
                    same_goal_trace: None,
                },
                selection: SelectionTrace {
                    selected_opportunity: Some(sleep),
                    selected_plan: None,
                    selected_plan_source: None,
                    goal_switch: None,
                    previous_goal: None,
                    plan_replacement: None,
                    snapshot_continuation: None,
                },
                portfolio: Some(PortfolioTrace {
                    slots: BTreeMap::from([
                        (
                            crate::SlotKind::NeedSurvival,
                            PortfolioSlotTrace {
                                goal_key: GoalKey::from(GoalKind::Sleep),
                                motive_score: 10,
                                feasibility: FeasibilityVerdict::Plausible,
                            },
                        ),
                        (
                            crate::SlotKind::EconomicOpportunity,
                            PortfolioSlotTrace {
                                goal_key: GoalKey::from(wash_goal()),
                                motive_score: 3,
                                feasibility: FeasibilityVerdict::Plausible,
                            },
                        ),
                    ]),
                    slots_attempted: 2,
                }),
                execution: ExecutionTrace {
                    enqueued_step: None,
                    revalidation_passed: None,
                    failure: None,
                },
                action_start_failures: Vec::new(),
                discrepancy_trace: Vec::new(),
                exhaustion_snapshot: Vec::new(),
                frame_transition: None,
                patrol_route: crate::PatrolRouteSnapshotTrace {
                    route: None,
                    current_waypoint: None,
                },
                selected_patrol_anchor: None,
                pursuit_invalidation: None,
            })),
            compiled_opportunities: Vec::new(),
            opportunity_compiler_load: Some(OpportunityCompilerLoad {
                compiled_count: 5,
                salience_floored: 1,
                learned_memory_damped: 2,
                cap_truncated: 3,
            }),
            snapshot_cache_counters: Some(SnapshotCacheCounters {
                cache_hit_count: 11,
                cache_miss_count: 4,
                cache_invalidation_count: 0,
            }),
            planning_state_cache_counters: Some(crate::PlanningStateCacheCounters {
                entities_at_hits: 3,
                entities_at_misses: 2,
                effective_place_hits: 5,
                effective_place_misses: 4,
                invalidations: 1,
            }),
            repair_attempts: Vec::new(),
            causal_link_cap_hits: Vec::new(),
        }
    }

    fn found_plan_attempt(goal: GoalKind, depth: usize) -> PlanAttemptTrace {
        PlanAttemptTrace {
            goal: GoalKey::from(goal),
            opportunity_anchor: OpportunityAnchor::None,
            outcome: PlanSearchOutcome::Found {
                steps: (0..depth)
                    .map(|index| PlannedStepSummary {
                        action_def_id: ActionDefId(u32::try_from(index).unwrap_or(0)),
                        action_name: format!("step-{index}"),
                        op_kind: crate::PlannerOpKind::Sleep,
                        targets: Vec::new(),
                        estimated_ticks: 1,
                        binding_strictness: None,
                    })
                    .collect(),
                terminal_kind: crate::PlanTerminalKind::GoalSatisfied,
            },
            goal_budget: worldwake_core::GoalPlanningBudget::TRAVEL_PURCHASE,
            strategic_budget: None,
            strategic_plan: None,
            tactical_goal: None,
            landmarks_extracted: 0,
            landmark_orderings: 0,
            target_belief_presence: TargetBeliefPresence::NotApplicable,
            method_trace: None,
            binding_rejections: Vec::new(),
            expansion_summaries: Vec::new(),
        }
    }

    fn repair_trace(
        chosen_kind: Option<RepairKind>,
        consumed: u16,
        total: u16,
    ) -> RepairAttemptTrace {
        RepairAttemptTrace {
            breach: worldwake_core::BreachSignature {
                goal_key: GoalKey::from(GoalKind::Sleep),
                invalidator: InvalidatorTag::TargetMoved,
                step_target: Some(entity(2)),
            },
            chosen_kind,
            rejected: Vec::<(RepairKind, RepairFailure)>::new(),
            budget_consumed: consumed,
            budget_total: total,
        }
    }

    fn ranked(opportunity: OpportunityKey) -> RankedGoalSummary {
        RankedGoalSummary {
            opportunity,
            priority_class: GoalPriorityClass::Background,
            motive_score: 10,
            ..RankedGoalSummary::default()
        }
    }

    fn opportunity(goal: GoalKind) -> OpportunityKey {
        OpportunityKey {
            goal_key: GoalKey::from(goal),
            anchor: OpportunityAnchor::None,
        }
    }

    fn wash_goal() -> GoalKind {
        GoalKind::Wash
    }

    fn testimony_summary(topic: TopicScope) -> TestimonyTrustSummary {
        TestimonyTrustSummary {
            source: entity(40),
            topic,
            trust: Permille::new_unchecked(300),
            observations: 2,
        }
    }

    fn emit_decision_payload(
        event_log: &mut EventLog,
        tick: Tick,
        tag: EventTag,
        payload: DecisionEventPayload,
    ) {
        let mut tags = BTreeSet::new();
        tags.insert(tag);
        event_log.emit(PendingEvent::from_payload(event_payload(
            tick,
            tags,
            Some(payload),
            None,
        )));
    }

    fn emit_blocker_recorded(event_log: &mut EventLog, tick: Tick, scope: BlockerScope) {
        emit_decision_payload(
            event_log,
            tick,
            EventTag::BlockerRecorded,
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent: entity(1),
                scope,
                discrepancy: None,
                blocking_fact: Some(BlockingFact::PatienceExhausted),
                expires_tick: Tick(tick.0 + 10),
                belief_snapshot: None,
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
        );
    }

    fn emit_contention_payload(event_log: &mut EventLog, tick: Tick, tag: EventTag) {
        let mut tags = BTreeSet::new();
        tags.insert(tag);
        event_log.emit(PendingEvent::from_payload(event_payload(
            tick,
            tags,
            None,
            Some(ContentionEventPayload {
                contested_affordance: worldwake_core::AffordanceKey {
                    facility: entity(30),
                    action: ActionDefId(1),
                },
                place: entity(40),
                resolution_rule: ContentionResolutionRule::ArrivalTime,
                claimants: vec![
                    ContentionClaimant {
                        agent: entity(1),
                        arrived_tick: Tick(tick.0.saturating_sub(2)),
                        queue_position: 1,
                        outcome: ClaimantOutcome::Granted,
                    },
                    ContentionClaimant {
                        agent: entity(2),
                        arrived_tick: Tick(tick.0.saturating_sub(1)),
                        queue_position: 2,
                        outcome: ClaimantOutcome::QueuedBehind,
                    },
                ],
                total_claimants: 2,
                winner: Some(entity(1)),
                at_tick: tick,
            }),
        )));
    }

    fn emit_plain_event(event_log: &mut EventLog, tick: Tick, tag: EventTag) {
        let mut tags = BTreeSet::new();
        tags.insert(tag);
        event_log.emit(PendingEvent::from_payload(event_payload(
            tick, tags, None, None,
        )));
    }

    fn event_payload(
        tick: Tick,
        tags: BTreeSet<EventTag>,
        decision_payload: Option<DecisionEventPayload>,
        contention_event_payload: Option<ContentionEventPayload>,
    ) -> EventPayload {
        EventPayload {
            tick,
            cause: CauseRef::SystemTick(tick),
            actor_id: Some(entity(1)),
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: Some(entity(99)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags,
            contention_event_payload,
            decision_payload,
            artifact_transition_payload: None,
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn payload_bearing_discrepancy_variants_are_grouped_by_discriminant() {
        let mut event_log = EventLog::new();
        emit_decision_payload(
            &mut event_log,
            Tick(1),
            EventTag::ReplanTriggered,
            DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                reason: ReplanReason::DiscrepancyRecorded {
                    discrepancy: Discrepancy::Omission(OmissionReason::OverBudget {
                        budget: 1,
                        candidates_seen: 2,
                    }),
                },
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
        );
        emit_decision_payload(
            &mut event_log,
            Tick(2),
            EventTag::ReplanTriggered,
            DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                agent: entity(1),
                goal_key: GoalKey::from(GoalKind::Sleep),
                reason: ReplanReason::DiscrepancyRecorded {
                    discrepancy: Discrepancy::Omission(OmissionReason::SalienceBelowFloor {
                        policy: worldwake_core::SaliencePolicy::PriorityWithNeedBoost,
                    }),
                },
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
        );

        let report = build_scenario_diagnostics(&[], &[], &[], &event_log, (Tick(0), Tick(3)));

        assert_eq!(
            report.revalidation_repair.invalidation_reasons,
            BTreeMap::from([(
                Discrepancy::Omission(OmissionReason::OverBudget {
                    budget: 0,
                    candidates_seen: 0,
                }),
                2,
            )])
        );
    }
}
