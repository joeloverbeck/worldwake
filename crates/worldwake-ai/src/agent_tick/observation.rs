use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    Blocker, BlockerKey, BlockerMemory, BlockingFact, CommodityKind, DecisionEventPayload,
    DiscrepancyMemory, EntityId, EventTag, ExpectationKindTag, ExpectationMismatchPayload, GoalKey,
    LearnedOpportunityMemory, MismatchDetail, OpportunityAnchor, PlanInvalidationReason, Quantity,
    RepairMemory, ReplanReason, Tick, UniqueItemKind,
};
use worldwake_sim::{
    ActionStartFailure, CommittedAction, RecipeRegistry, ReplanNeeded, RuntimeBeliefView,
    TickInputError,
};

use crate::decision_trace::CandidateDampingEntry;
use crate::failure_handling::ExecutionFailure;
use crate::knowledge_path::KnowledgePath;
use crate::opportunity_compiler::{
    Opportunity, PerceivedOpportunityIndex, build_perceived_opportunity_index,
    compile_opportunities,
};
use crate::plan_step_expectations::{
    fulfill_plan_step_expectations, persist_expectation_store_update,
};
use crate::ranking::rank_candidates_with_memories_and_testimony_reliability;
use crate::{
    AgendaEntry, AgentDecisionRuntime, DecisionContext, ExpectationFailureCause,
    ExpectationFailurePhase, GoalKindPlannerExt, OpportunityExpectationFailureIncident,
    PlannedStep, authoritative_target, clear_resolved_failures,
};
use worldwake_core::{ContentionIntents, QueuedContentionIntent};

use super::{
    AgentTickContext, AssumptionRefContext, advance_completed_step,
    apply_step_materialization_bindings, committed_action_for_step, current_step,
    decisive_evidence_from_mismatch_detail, emit_decision_event, handle_current_step_failure,
    plan_finished, runtime_belief_view,
};

#[derive(Clone, Copy)]
pub(crate) struct ReadPhaseContext<'a> {
    pub(super) recipe_registry: &'a RecipeRegistry,
    pub(super) utility: &'a worldwake_core::UtilityProfile,
    pub(super) tick: Tick,
    pub(super) travel_horizon: u8,
    pub(super) structural_block_ticks: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct InFlightReconciliation<'a> {
    pub(super) replan_signals: &'a [&'a ReplanNeeded],
    pub(super) start_failures: &'a [ActionStartFailure],
    pub(super) committed_actions: &'a [CommittedAction],
}

#[derive(Clone, Copy)]
pub(crate) struct CompletedPlanSummary {
    pub(super) goal_key: worldwake_core::GoalKey,
    pub(super) terminal_kind: crate::PlanTerminalKind,
    pub(super) step_index: u16,
}

#[derive(Default)]
pub(crate) struct ReconciliationResult {
    pub(super) completed_plan: Option<CompletedPlanSummary>,
    pub(super) plan_invalidation: Option<(GoalKey, PlanInvalidationReason)>,
    pub(super) replan_trigger: Option<(GoalKey, ReplanReason)>,
}

/// Result of the read phase, preserving trace-relevant data alongside ranked candidates.
pub(crate) struct ReadPhaseResult {
    /// Candidate offers emitted during generation in generation order.
    pub(super) offered: Vec<crate::candidate_generation::CandidateOfferDiagnostic>,
    pub(super) ranked: Vec<AgendaEntry>,
    /// Generated candidate keys (before ranking filter).
    pub(super) generated_keys: Vec<worldwake_core::OpportunityKey>,
    /// Typed candidate-evidence provenance keyed by generated goal.
    pub(super) candidate_evidence: Vec<crate::CandidateEvidenceTrace>,
    /// Candidate source attribution keyed by generated goal.
    pub(super) candidate_sources:
        BTreeMap<worldwake_core::OpportunityKey, crate::decision_trace::CandidateSource>,
    /// Compiled per-tick opportunities available to downstream S138 consumers.
    #[allow(dead_code)]
    pub(super) opportunities: Vec<Opportunity>,
    /// Dense index over the compiled opportunities for same-tick consumers.
    #[allow(dead_code)]
    pub(super) opportunity_index: PerceivedOpportunityIndex,
    /// Per-agent load counters for the compiler pass.
    #[allow(dead_code)]
    pub(super) opportunity_compiler_load: crate::decision_trace::OpportunityCompilerLoad,
    /// Desire-level diagnostics for goals whose emitted sibling opportunities
    /// were all filtered out as blocked before ranking.
    pub(super) fully_blocked_desires: Vec<crate::DesireFullyBlocked>,
    /// Aggregate reachable-place count across acquisition-place searches.
    pub(super) places_reachable: u32,
    /// Aggregate kept-place count after belief gating across acquisition-place searches.
    pub(super) places_after_belief_filter: u32,
    /// Candidate opportunities suppressed before commitment.
    pub(super) suppressed: Vec<crate::candidate_generation::CandidateSuppressionDiagnostic>,
    /// Emitted goals whose ranking score was reduced by soft damping.
    pub(super) damped: Vec<CandidateDampingEntry>,
    /// Goals with zero motive score.
    pub(super) zero_motive: Vec<worldwake_core::GoalKey>,
    /// Political goals omitted before emission due to hard gates.
    pub(super) omitted_political: Vec<crate::PoliticalCandidateOmission>,
    /// Bandit goals omitted before emission due to local candidate gates.
    pub(super) omitted_bandit: Vec<crate::BanditCandidateOmission>,
    /// Social goals omitted before emission due to resend suppression.
    pub(super) omitted_social: Vec<crate::SocialCandidateOmission>,
    /// Testimony goals omitted before emission due to learned reliability.
    pub(super) omitted_testimony: Vec<crate::TestimonyCandidateOmission>,
    /// Violation detection pass skipped due to missing prerequisites.
    pub(super) omitted_violation_detection: Vec<crate::ViolationDetectionOmission>,
    /// Shared decision context built once from beliefs for ranking + interrupts.
    pub(super) decision_context: DecisionContext,
    /// When a pursuit plan was invalidated, records the reason.
    pub(super) pursuit_invalidation: Option<crate::PursuitInvalidationReason>,
    /// Source-backed expectation contradictions detected during candidate generation/read phase.
    pub(super) pending_source_reliability_failures: Vec<OpportunityExpectationFailureIncident>,
    /// Need-specific tracker resets detected during candidate generation.
    pub(super) pending_acquisition_exhaustion_resets:
        std::collections::BTreeSet<worldwake_core::HomeostaticNeedId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExpectationMismatchContext<'a> {
    pub(super) expectation_kind: Option<ExpectationKindTag>,
    pub(super) mismatch_detail: Option<MismatchDetail>,
    pub(super) assumption_refs: AssumptionRefContext<'a>,
    pub(super) max_decisive_evidence: u8,
}

impl Default for ExpectationMismatchContext<'_> {
    fn default() -> Self {
        Self {
            expectation_kind: None,
            mismatch_detail: None,
            assumption_refs: AssumptionRefContext::new(&[], 0),
            max_decisive_evidence: 0,
        }
    }
}

pub(super) fn emit_expectation_mismatch(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    goal_key: worldwake_core::GoalKey,
    step_index: usize,
    step: &PlannedStep,
    context: ExpectationMismatchContext<'_>,
) {
    let decisive = decisive_evidence_from_mismatch_detail(
        agent,
        context.mismatch_detail,
        tick,
        context.max_decisive_evidence,
    );
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::ExpectationMismatch,
        DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
            agent,
            goal_key,
            step_index: step_index.try_into().expect("step index exceeds u16"),
            expected_materializations: step
                .expected_materializations
                .iter()
                .map(|expected| expected.tag)
                .collect(),
            expectation_kind: context.expectation_kind,
            mismatch_detail: context.mismatch_detail,
            decisive_beliefs: decisive.beliefs,
            decisive_records: decisive.records,
            decisive_world_observations: decisive.world_observations,
            assumptions: context.assumption_refs.to_refs(),
        }),
    );
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn refresh_runtime_for_read_phase(
    world: &worldwake_core::World,
    scheduler: &worldwake_sim::Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    violation_memory: &mut worldwake_core::ViolationMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
    tracing: bool,
) -> ReadPhaseResult {
    let mut discrepancy_memory = DiscrepancyMemory::default();
    refresh_runtime_for_read_phase_with_memories(
        world,
        scheduler,
        action_defs,
        runtime,
        active_goal,
        facility_intents,
        blocked_memory,
        &mut discrepancy_memory,
        violation_memory,
        &RepairMemory::default(),
        &LearnedOpportunityMemory::default(),
        &crate::EffectSchemaIndex::default(),
        agent,
        replan_signals,
        phase,
        tracing,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refresh_runtime_for_read_phase_with_memories(
    world: &worldwake_core::World,
    scheduler: &worldwake_sim::Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    violation_memory: &mut worldwake_core::ViolationMemory,
    repair_memory: &RepairMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
    effect_schema_index: &crate::EffectSchemaIndex,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
    tracing: bool,
) -> ReadPhaseResult {
    // One authoritative read view covers blocker cleanup, snapshot dirtiness, and ranking.
    let view = runtime_belief_view(agent, world, scheduler, action_defs, phase.recipe_registry);
    let (opportunities, opportunity_compiler_load) =
        compile_opportunities(agent, &view, effect_schema_index);
    let candidate_opportunities: Vec<Opportunity> = opportunities
        .iter()
        .filter(|opportunity| Some(opportunity.key.goal_key) != active_goal)
        .cloned()
        .collect();
    let before = blocked_memory.clone();
    let queue_transition_changed = handle_facility_queue_transitions(
        &view,
        runtime,
        facility_intents,
        blocked_memory,
        agent,
        phase.tick,
        phase,
    );
    clear_resolved_failures(&view, agent, blocked_memory, discrepancy_memory, phase.tick);
    let blocked_changed_from_cleanup = *blocked_memory != before;
    let snapshot_domains =
        observation_snapshot_changed(&view, agent, active_goal, runtime, phase.recipe_registry);
    let queue_patience_exhausted = facility_queue_patience_exhausted(&view, agent, phase.tick);

    // Accumulate all dirty bits directly on runtime.dirty — no dual-tracking.
    if runtime.current_plan.is_none() {
        runtime.dirty.insert(crate::DirtySet::NO_PLAN);
    }
    if plan_finished(runtime) {
        runtime.dirty.insert(crate::DirtySet::PLAN_FINISHED);
    }
    if !replan_signals.is_empty() {
        runtime.dirty.insert(crate::DirtySet::REPLAN_SIGNAL);
    }
    if queue_transition_changed {
        runtime.dirty.insert(crate::DirtySet::QUEUE_TRANSITION);
    }
    if blocked_changed_from_cleanup {
        runtime.dirty.insert(crate::DirtySet::BLOCKER_CLEANUP);
    }
    runtime.dirty.insert(snapshot_domains);
    if queue_patience_exhausted {
        runtime.dirty.insert(crate::DirtySet::QUEUE_PATIENCE);
    }

    // Pursuit plan invalidation: if the active plan is a remote pursuit
    // (Travel + Attack for RaidTarget/EngageHostile), check whether the
    // underlying belief assumptions still hold. If not, clear the plan
    // and force replanning.
    let pursuit_invalidation = runtime
        .current_plan
        .as_ref()
        .and_then(|plan| crate::is_pursuit_plan_invalid(&view, agent, plan, phase.tick));
    if pursuit_invalidation.is_some() {
        runtime.current_plan = None;
        runtime.current_step_index = 0;
        runtime.materialization_bindings.clear();
        facility_intents.intents.clear();
        runtime.dirty.insert(crate::DirtySet::REPLAN_SIGNAL);
    }

    let mut candidates =
        crate::candidate_generation::generate_candidates_with_current_plan_with_memories_with_travel_horizon_and_opportunities_and_testimony_reliability(
            &view,
            agent,
            blocked_memory,
            discrepancy_memory,
            violation_memory,
            phase.recipe_registry,
            phase.tick,
            phase.travel_horizon,
            tracing,
            runtime.current_plan.as_ref(),
            &candidate_opportunities,
            &runtime.testimony_reliability,
        );
    let opportunity_index = build_perceived_opportunity_index(
        opportunities
            .iter()
            .filter(|opportunity| {
                matches!(
                    candidates.diagnostics.sources.get(&opportunity.key),
                    Some(crate::decision_trace::CandidateSource::OpportunityCompiler)
                )
            })
            .cloned()
            .collect(),
    );
    reinstate_current_plan_candidate(&mut candidates, runtime, active_goal);
    candidates.pending_source_reliability_failures.extend(
        pending_local_source_reliability_failures(
            &view,
            agent,
            runtime.current_plan.as_ref(),
            phase.tick,
        ),
    );

    // Apply deferred violation records from candidate generation.
    for pending in &candidates.pending_violations {
        let recorded_id =
            violation_memory.record(pending.kind.clone(), pending.observed_tick, pending.ttl);
        debug_assert_eq!(recorded_id, pending.id);
    }
    apply_pending_discrepancies(
        discrepancy_memory,
        &candidates.pending_discrepancies,
        phase.structural_block_ticks,
    );

    let generated_keys = candidates
        .candidates
        .iter()
        .map(|c| worldwake_core::OpportunityKey {
            goal_key: c.key,
            anchor: c.anchor,
        })
        .collect();
    let candidate_evidence = candidates.diagnostics.evidence.values().cloned().collect();
    let dc = crate::build_decision_context(&view, agent);
    let outcome = rank_candidates_with_memories_and_testimony_reliability(
        &candidates.candidates,
        &view,
        agent,
        phase.tick,
        phase.utility,
        dc,
        repair_memory,
        learned_opportunity_memory,
        &runtime.testimony_reliability,
    );
    let mut ranked = outcome.ranked;
    crate::ranking::apply_pending_source_reliability_failures(
        &mut ranked,
        &crate::ranking::PendingSourceReliabilityInputs {
            view: &view,
            agent,
            current_tick: phase.tick,
            utility: phase.utility,
            decision_context: dc,
            repair_memory,
            learned_opportunity_memory,
            testimony_reliability: &runtime.testimony_reliability,
        },
        &candidates.pending_source_reliability_failures,
    );

    ReadPhaseResult {
        offered: candidates.diagnostics.offers,
        ranked,
        generated_keys,
        candidate_evidence,
        candidate_sources: candidates.diagnostics.sources,
        opportunities,
        opportunity_index,
        opportunity_compiler_load,
        fully_blocked_desires: candidates.diagnostics.fully_blocked_desires,
        places_reachable: candidates.diagnostics.places_reachable,
        places_after_belief_filter: candidates.diagnostics.places_after_belief_filter,
        suppressed: {
            let mut suppressed = candidates.diagnostics.suppressed;
            suppressed.extend(outcome.suppressed);
            suppressed
        },
        damped: outcome.damped,
        zero_motive: outcome.zero_motive,
        omitted_political: candidates.diagnostics.omitted_political,
        omitted_bandit: candidates.diagnostics.omitted_bandit,
        omitted_social: candidates.diagnostics.omitted_social,
        omitted_testimony: candidates.diagnostics.omitted_testimony,
        omitted_violation_detection: candidates.diagnostics.omitted_violation_detection,
        decision_context: dc,
        pursuit_invalidation,
        pending_source_reliability_failures: candidates.pending_source_reliability_failures,
        pending_acquisition_exhaustion_resets: candidates.pending_acquisition_exhaustion_resets,
    }
}

fn apply_pending_discrepancies(
    discrepancy_memory: &mut DiscrepancyMemory,
    pending_discrepancies: &[crate::candidate_generation::PendingDiscrepancyRecord],
    structural_block_ticks: u32,
) {
    for pending in pending_discrepancies {
        discrepancy_memory.record(worldwake_core::DiscrepancyEntry {
            scope: pending.scope,
            discrepancy: pending.discrepancy,
            observed_tick: pending.observed_tick,
            expires_tick: Tick(
                pending
                    .observed_tick
                    .0
                    .saturating_add(u64::from(structural_block_ticks)),
            ),
            clearing_condition: pending.clearing_condition,
            // read-phase inference from PendingDiscrepancyRecord; no triggering event in scope
            source: worldwake_core::DiscrepancySource::ReadPhaseInference,
        });
    }
}

fn pending_local_source_reliability_failures(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    current_plan: Option<&crate::PlannedPlan>,
    tick: Tick,
) -> Vec<OpportunityExpectationFailureIncident> {
    let Some(current_place) = view.effective_place(agent) else {
        return Vec::new();
    };
    if view.preference_profile(agent).is_none() {
        return Vec::new();
    }
    let Some(plan) = current_plan else {
        return Vec::new();
    };
    if plan.opportunity.anchor != OpportunityAnchor::Place(current_place) {
        return Vec::new();
    }
    let (Some(source_key), Some(expectation_kind)) = (plan.committed_source, plan.expectation_kind)
    else {
        return Vec::new();
    };
    if source_key.entity != current_place
        && !view
            .colocated_entities(agent)
            .value
            .contains(&source_key.entity)
    {
        return vec![OpportunityExpectationFailureIncident {
            opportunity: plan.opportunity,
            source: source_key,
            expectation_kind,
            detected_at_tick: tick,
            phase: ExpectationFailurePhase::Observation,
            cause: ExpectationFailureCause::SourceAbsentLocally,
        }];
    }
    if view.locally_observed_commodity_quantity(agent, source_key.entity, source_key.commodity)
        != Quantity(0)
    {
        return Vec::new();
    }
    vec![OpportunityExpectationFailureIncident {
        opportunity: plan.opportunity,
        source: source_key,
        expectation_kind,
        detected_at_tick: tick,
        phase: ExpectationFailurePhase::Observation,
        cause: ExpectationFailureCause::SourceDepletedLocally,
    }]
}

fn reinstate_current_plan_candidate(
    candidates: &mut crate::candidate_generation::CandidateGenerationResult,
    runtime: &AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
) {
    let Some(plan) = runtime.current_plan.as_ref() else {
        return;
    };
    let opportunity = plan.opportunity;
    if Some(opportunity.goal_key) != active_goal {
        return;
    }

    if candidates.candidates.iter().any(|candidate| {
        candidate.key == opportunity.goal_key && candidate.anchor == opportunity.anchor
    }) {
        return;
    }
    if !candidates
        .candidates
        .iter()
        .any(|candidate| candidate.key == opportunity.goal_key)
    {
        return;
    }

    let mut evidence_entities = BTreeSet::new();
    let mut evidence_places = BTreeSet::new();
    match opportunity.anchor {
        worldwake_core::OpportunityAnchor::Place(place) => {
            evidence_places.insert(place);
        }
        worldwake_core::OpportunityAnchor::Entity(entity) => {
            evidence_entities.insert(entity);
        }
        worldwake_core::OpportunityAnchor::None => {
            if let Some(place) = opportunity.goal_key.place {
                evidence_places.insert(place);
            }
            if let Some(entity) = opportunity.goal_key.entity {
                evidence_entities.insert(entity);
            }
        }
    }

    candidates.candidates.push(crate::GoalOffer {
        key: opportunity.goal_key,
        anchor: opportunity.anchor,
        evidence_entities,
        evidence_places,
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: crate::motive_source_mapping::derive_default_motive_sources(
            &opportunity.goal_key.kind,
            &opportunity.anchor,
            worldwake_core::Tick(0),
        ),
        acquisition_quantity: None,
    });
    candidates
        .diagnostics
        .evidence
        .entry(opportunity)
        .or_insert(crate::CandidateEvidenceTrace {
            opportunity,
            contributors: Vec::new(),
            exclusions: Vec::new(),
            knowledge_path: KnowledgePath::default(),
            legality: None,
            pursuit: None,
            artifact_axes: None,
        });
}

pub(super) fn handle_facility_queue_transitions(
    view: &dyn RuntimeBeliefView,
    runtime: &AgentDecisionRuntime,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    agent: EntityId,
    tick: Tick,
    phase: ReadPhaseContext<'_>,
) -> bool {
    let previous_place = runtime.last_effective_place;
    let current_place = view.effective_place(agent);
    let current_signature = facility_access_signature(view, agent);
    let current_by_facility = current_signature
        .iter()
        .copied()
        .map(|(facility, queued, grant)| (facility, (queued, grant)))
        .collect::<BTreeMap<_, _>>();
    let mut changed = false;

    for (facility, was_queued, previous_grant) in runtime.last_facility_access_signature.clone() {
        let current = current_by_facility.get(&facility).copied();
        let now_queued = current.is_some_and(|(queued, _)| queued);
        let now_granted = current.and_then(|(_, grant)| grant);

        if was_queued && !now_queued && now_granted.is_none() {
            if previous_place == current_place {
                let fallback_intent = runtime.current_plan.as_ref().and_then(|plan| {
                    let intended_action = view
                        .facility_grant(facility)
                        .map(|grant| grant.intended_action)
                        .or_else(|| {
                            plan.steps.iter().find_map(|step| {
                                let matches_facility = step
                                    .targets
                                    .first()
                                    .copied()
                                    .and_then(crate::authoritative_target)
                                    == Some(facility);
                                if !matches_facility {
                                    return None;
                                }
                                match step.op_kind {
                                    crate::PlannerOpKind::QueueForFacilityUse => step
                                        .payload_override
                                        .as_ref()
                                        .and_then(
                                            worldwake_sim::ActionPayload::as_queue_for_facility_use,
                                        )
                                        .map(|payload| payload.intended_action),
                                    crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
                                        Some(step.def_id)
                                    }
                                    _ => None,
                                }
                            })
                        })?;
                    Some(QueuedContentionIntent {
                        goal_key: plan.goal,
                        intended_action,
                    })
                });
                if let Some(intent) = facility_intents
                    .intents
                    .remove(&facility)
                    .or(fallback_intent)
                {
                    let scope = BlockerKey {
                        goal_key: intent.goal_key,
                        place: current_place,
                        target: Some(facility),
                        action_def: Some(intent.intended_action),
                    }
                    .into();
                    blocked_memory.record(Blocker {
                        scope,
                        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                        diagnostic_context: None,
                        observed_tick: tick,
                        expires_tick: tick + u64::from(phase.structural_block_ticks),
                        clearing_condition:
                            worldwake_core::BlockerClearingCondition::for_scope_and_fact(
                                scope,
                                BlockingFact::ExclusiveFacilityUnavailable,
                                worldwake_core::BlockerClearingCondition::TtlOnly,
                            ),
                        baseline_snapshot: None,
                        source: worldwake_core::BlockerSource::Inferred,
                    });
                    changed = true;
                }
            } else if facility_intents.intents.remove(&facility).is_some() {
                changed = true;
            }
        }

        if previous_grant.is_some() && now_granted.is_none() {
            changed |= facility_intents.intents.remove(&facility).is_some();
        }
    }

    changed
}

#[allow(clippy::too_many_arguments)]
pub(super) fn reconcile_in_flight_state(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<crate::AgendaEntry>,
    jc: &mut Option<worldwake_core::IntentionFrame>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    active_action: Option<&worldwake_sim::ActionInstance>,
    agent: EntityId,
    reconciliation: InFlightReconciliation<'_>,
) -> Result<ReconciliationResult, TickInputError> {
    if !runtime.step_in_flight {
        return Ok(ReconciliationResult::default());
    }
    if active_action.is_some() {
        return Ok(ReconciliationResult::default());
    }

    let failed_signal = reconciliation.replan_signals.first().copied();
    let Some(step) = current_step(runtime).cloned() else {
        runtime.step_in_flight = false;
        return Ok(ReconciliationResult::default());
    };
    let goal_key = active_goal
        .as_ref()
        .map(|ag| ag.key.goal_key)
        .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
        .expect("in-flight step must have a current goal");

    if let Some(signal) = failed_signal {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            Some(ExecutionFailure::Replan(signal)),
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    }

    if let Some(start_failure) = matching_start_failure(&step, reconciliation.start_failures) {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            Some(ExecutionFailure::Start(start_failure)),
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    }

    let Some(committed_action) = committed_action_for_step(&step, reconciliation.committed_actions)
    else {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            None,
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    };
    let completed_plan = runtime.current_plan.as_ref().and_then(|plan| {
        let next_index = runtime.current_step_index.checked_add(1)?;
        (next_index >= plan.steps.len()
            && matches!(
                plan.terminal_kind,
                crate::PlanTerminalKind::GoalSatisfied | crate::PlanTerminalKind::CombatCommitment
            ))
        .then_some(CompletedPlanSummary {
            goal_key: plan.goal,
            terminal_kind: plan.terminal_kind,
            step_index: runtime
                .current_step_index
                .try_into()
                .expect("step index exceeds u16"),
        })
    });
    reconcile_committed_facility_queue_intents(runtime, facility_intents, Some(goal_key), &step);
    if apply_step_materialization_bindings(runtime, &step, &committed_action.outcome).is_err() {
        emit_expectation_mismatch(
            ctx.event_log,
            ctx.tick,
            agent,
            goal_key,
            runtime.current_step_index,
            &step,
            ExpectationMismatchContext {
                assumption_refs: AssumptionRefContext::from_frame(
                    jc.as_ref(),
                    ctx.cognitive.decision_history_alternatives,
                    runtime.current_plan.as_ref(),
                ),
                max_decisive_evidence: ctx.cognitive.decision_history_alternatives,
                ..ExpectationMismatchContext::default()
            },
        );
        let invalidation_reason = PlanInvalidationReason::ExpectationMismatch {
            step_index: runtime
                .current_step_index
                .try_into()
                .expect("step index exceeds u16"),
        };
        let _ = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            None,
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: Some((goal_key, invalidation_reason)),
            replan_trigger: Some((
                goal_key,
                ReplanReason::PlanInvalidated {
                    reason: invalidation_reason,
                },
            )),
        });
    }

    runtime.step_in_flight = false;
    let completed_step_index = runtime
        .current_step_index
        .try_into()
        .expect("step index exceeds u16");
    let _ = persist_expectation_store_update(ctx.world, ctx.event_log, agent, ctx.tick, |store| {
        fulfill_plan_step_expectations(store, completed_step_index)
    })?;
    *jc = advance_completed_step(
        runtime,
        active_goal,
        facility_intents,
        jc.as_ref(),
        step.op_kind,
        ctx.tick,
    );
    Ok(ReconciliationResult {
        completed_plan,
        plan_invalidation: None,
        replan_trigger: None,
    })
}

fn matching_start_failure<'a>(
    step: &PlannedStep,
    start_failures: &'a [ActionStartFailure],
) -> Option<&'a ActionStartFailure> {
    start_failures
        .iter()
        .find(|failure| failure.def_id == step.def_id)
}

fn reconcile_committed_facility_queue_intents(
    runtime: &AgentDecisionRuntime,
    facility_intents: &mut ContentionIntents,
    active_goal: Option<worldwake_core::GoalKey>,
    step: &PlannedStep,
) {
    let Some(facility) = step.targets.first().copied().and_then(authoritative_target) else {
        return;
    };

    match step.op_kind {
        crate::PlannerOpKind::QueueForFacilityUse => {
            let Some(goal_key) =
                active_goal.or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
            else {
                return;
            };
            let Some(payload) = step
                .payload_override
                .as_ref()
                .and_then(worldwake_sim::ActionPayload::as_queue_for_facility_use)
            else {
                return;
            };
            facility_intents.intents.insert(
                facility,
                QueuedContentionIntent {
                    goal_key,
                    intended_action: payload.intended_action,
                },
            );
        }
        crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
            facility_intents.intents.remove(&facility);
        }
        crate::PlannerOpKind::Travel
        | crate::PlannerOpKind::Patrol
        | crate::PlannerOpKind::Sleep
        | crate::PlannerOpKind::Relieve
        | crate::PlannerOpKind::EstablishCamp
        | crate::PlannerOpKind::Trade
        | crate::PlannerOpKind::Consume
        | crate::PlannerOpKind::Wash
        | crate::PlannerOpKind::CleanWashBasin
        | crate::PlannerOpKind::EmptyLatrine
        | crate::PlannerOpKind::Heal
        | crate::PlannerOpKind::MoveCargo
        | crate::PlannerOpKind::DropItem
        | crate::PlannerOpKind::Loot
        | crate::PlannerOpKind::Bury
        | crate::PlannerOpKind::Tell
        | crate::PlannerOpKind::ConsultRecord
        | crate::PlannerOpKind::Attack
        | crate::PlannerOpKind::Defend
        | crate::PlannerOpKind::Bribe
        | crate::PlannerOpKind::Threaten
        | crate::PlannerOpKind::Accuse
        | crate::PlannerOpKind::Fine
        | crate::PlannerOpKind::Exile
        | crate::PlannerOpKind::DeclareSupport
        | crate::PlannerOpKind::PressForceClaim
        | crate::PlannerOpKind::YieldForceClaim
        | crate::PlannerOpKind::Investigate
        | crate::PlannerOpKind::AskWitness
        | crate::PlannerOpKind::SearchPlace
        | crate::PlannerOpKind::AskAboutPerson
        | crate::PlannerOpKind::ReportMissing
        | crate::PlannerOpKind::EscortToSafety
        | crate::PlannerOpKind::ReportFound
        | crate::PlannerOpKind::ClaimBounty
        | crate::PlannerOpKind::WithdrawBounty
        | crate::PlannerOpKind::PostBounty
        | crate::PlannerOpKind::PostNotice
        | crate::PlannerOpKind::StaffMarket
        | crate::PlannerOpKind::StockManagement => {}
    }
}

pub(super) fn observation_snapshot_changed(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    active_goal: Option<worldwake_core::GoalKey>,
    runtime: &AgentDecisionRuntime,
    recipe_registry: &RecipeRegistry,
) -> crate::DirtySet {
    let mut result = crate::DirtySet::default();
    let current_commodity_signature = commodity_signature(view, agent);
    let commodity_filter = active_goal
        .map(|goal| goal.kind.relevant_observed_commodities(recipe_registry))
        .or_else(|| {
            runtime.current_plan.as_ref().map(|plan| {
                plan.goal
                    .kind
                    .relevant_observed_commodities(recipe_registry)
            })
        });
    if runtime.last_effective_place != view.effective_place(agent) {
        result.insert(crate::DirtySet::POSITION);
    }
    if runtime.last_needs != view.homeostatic_needs(agent) {
        result.insert(crate::DirtySet::NEEDS);
    }
    if runtime.last_wounds != view.wounds(agent) {
        result.insert(crate::DirtySet::WOUNDS);
    }
    if filtered_commodity_signature(&runtime.last_commodity_signature, commodity_filter.as_ref())
        != filtered_commodity_signature(&current_commodity_signature, commodity_filter.as_ref())
    {
        result.insert(crate::DirtySet::COMMODITY);
    }
    if runtime.last_unique_item_signature != unique_item_signature(view, agent) {
        result.insert(crate::DirtySet::UNIQUE_ITEMS);
    }
    if runtime.last_facility_access_signature != facility_access_signature(view, agent) {
        result.insert(crate::DirtySet::FACILITIES);
    }
    if runtime.last_patrol_route != view.patrol_route(agent) {
        result.insert(crate::DirtySet::PATROL_ROUTE);
    }
    result
}

pub(super) fn update_runtime_observation_snapshot(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &mut AgentDecisionRuntime,
) {
    runtime.last_effective_place = view.effective_place(agent);
    runtime.last_needs = view.homeostatic_needs(agent);
    runtime.last_wounds = view.wounds(agent);
    runtime.last_commodity_signature = commodity_signature(view, agent);
    runtime.last_unique_item_signature = unique_item_signature(view, agent);
    runtime.last_facility_access_signature = facility_access_signature(view, agent);
    runtime.last_patrol_route = view.patrol_route(agent);
    runtime.last_in_transit = view.in_transit_state(agent).is_some();
}

pub(super) fn facility_access_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(EntityId, bool, Option<worldwake_core::ActionDefId>)> {
    let Some(place) = view.effective_place(agent) else {
        return Vec::new();
    };

    view.entities_at(place)
        .into_iter()
        .filter(|entity| view.has_contention_policy(*entity))
        .filter_map(|facility| {
            let queued = view.facility_queue_position(facility, agent).is_some();
            let matching_grant = view
                .facility_grant(facility)
                .and_then(|grant| (grant.actor == agent).then_some(grant.intended_action));
            (queued || matching_grant.is_some()).then_some((facility, queued, matching_grant))
        })
        .collect()
}

pub(super) fn facility_queue_patience_exhausted(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    tick: Tick,
) -> bool {
    let Some(limit) = view.facility_queue_patience_ticks(agent) else {
        return false;
    };
    let Some(place) = view.effective_place(agent) else {
        return false;
    };

    view.entities_at(place).into_iter().any(|facility| {
        if !view.has_contention_policy(facility) {
            return false;
        }
        if view
            .facility_grant(facility)
            .is_some_and(|grant| grant.actor == agent)
        {
            return false;
        }
        view.facility_queue_join_tick(facility, agent)
            .is_some_and(|queued_at| tick >= queued_at + u64::from(limit.get()))
    })
}

pub(super) fn commodity_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(CommodityKind, Quantity)> {
    CommodityKind::ALL
        .into_iter()
        .filter_map(|commodity| {
            let quantity = view.commodity_quantity(agent, commodity);
            (quantity > Quantity(0)).then_some((commodity, quantity))
        })
        .collect()
}

pub(super) fn filtered_commodity_signature(
    signature: &[(CommodityKind, Quantity)],
    relevant: Option<&Option<std::collections::BTreeSet<CommodityKind>>>,
) -> Vec<(CommodityKind, Quantity)> {
    match relevant {
        Some(Some(relevant)) => signature
            .iter()
            .copied()
            .filter(|(commodity, _)| relevant.contains(commodity))
            .collect(),
        Some(None) | None => signature.to_vec(),
    }
}

pub(super) fn unique_item_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(UniqueItemKind, u32)> {
    UniqueItemKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = view.unique_item_count(agent, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::AssumptionRefContext;
    use super::{
        ExpectationMismatchContext, apply_pending_discrepancies, emit_expectation_mismatch,
        pending_local_source_reliability_failures, reinstate_current_plan_candidate,
    };
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, ExpectationFailureCause, ExpectationFailurePhase,
        ExpectedMaterialization, GoalOffer, HypotheticalEntityId,
        OpportunityExpectationFailureIncident, OpportunityExpectationKind, PlanTerminalKind,
        PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef,
        candidate_generation::{
            CandidateGenerationDiagnostics, CandidateGenerationResult, PendingDiscrepancyRecord,
        },
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, BlockerKey, BlockerReason, CauseRef, CommodityKind,
        ControlSource, DecisionEventPayload, Discrepancy, DiscrepancyClearing, DiscrepancyMemory,
        DiscrepancySource, EntityId, EventLog, EventTag, EventView, FrameAssumption, GoalKey,
        GoalKind, HomeostaticNeedId, MaterializationTag, OpportunityAnchor, Quantity,
        ResourceSource, Tick, VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
    };
    use worldwake_sim::PerAgentBeliefView;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
    }

    #[test]
    fn apply_pending_discrepancies_records_artifact_actionability_discrepancy() {
        let artifact = entity(20);
        let goal_key = GoalKey::from(GoalKind::FulfillBounty { bounty: artifact });
        let blocker_key = BlockerKey {
            goal_key,
            place: None,
            target: Some(artifact),
            action_def: None,
        };
        let pending = PendingDiscrepancyRecord {
            scope: blocker_key.into(),
            discrepancy: Discrepancy::ArtifactNotActionable {
                artifact,
                reason: BlockerReason::LegalEffectExpired,
            },
            observed_tick: Tick(7),
            clearing_condition: DiscrepancyClearing::ReobservationOf { target: artifact },
        };
        let mut memory = DiscrepancyMemory::default();

        apply_pending_discrepancies(&mut memory, &[pending], 12);

        let entry = memory
            .entries
            .get(&blocker_key.into())
            .expect("pending discrepancy should be recorded");
        assert_eq!(entry.discrepancy, pending.discrepancy);
        assert_eq!(entry.observed_tick, Tick(7));
        assert_eq!(entry.expires_tick, Tick(19));
        assert_eq!(entry.source, DiscrepancySource::ReadPhaseInference);
        assert_eq!(
            entry.clearing_condition,
            DiscrepancyClearing::ReobservationOf { target: artifact }
        );
    }

    #[test]
    fn reinstate_current_plan_candidate_restores_missing_committed_opportunity() {
        let market = entity(10);
        let inn = entity(11);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![PlanningEntityRef::Authoritative(market)],
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 3,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };

        let mut candidates = CandidateGenerationResult {
            candidates: vec![GoalOffer {
                key: goal,
                anchor: OpportunityAnchor::Place(inn),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::from([inn]),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }],
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_discrepancies: Vec::new(),
            pending_source_reliability_failures: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(goal));

        assert!(candidates.candidates.iter().any(|candidate| {
            candidate.key == goal && candidate.anchor == OpportunityAnchor::Place(market)
        }));
        assert!(
            candidates
                .diagnostics
                .evidence
                .contains_key(&worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Place(market),
                })
        );
    }

    #[test]
    fn reinstate_current_plan_candidate_skips_when_active_goal_differs() {
        let market = entity(10);
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let other_goal = GoalKey::from(GoalKind::Sleep);
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: committed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            committed_goal,
            Vec::new(),
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };
        let mut candidates = CandidateGenerationResult {
            candidates: Vec::new(),
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_discrepancies: Vec::new(),
            pending_source_reliability_failures: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(other_goal));

        assert!(candidates.candidates.is_empty());
        assert!(candidates.diagnostics.evidence.is_empty());
    }

    #[test]
    fn reinstate_current_plan_candidate_skips_when_goal_has_no_live_siblings() {
        let market = entity(10);
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: committed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            committed_goal,
            Vec::new(),
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };
        let mut candidates = CandidateGenerationResult {
            candidates: vec![GoalOffer {
                key: GoalKey::from(GoalKind::Sleep),
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }],
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_discrepancies: Vec::new(),
            pending_source_reliability_failures: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(committed_goal));

        assert_eq!(candidates.candidates.len(), 1);
        assert_eq!(candidates.candidates[0].key, GoalKey::from(GoalKind::Sleep));
        assert!(candidates.diagnostics.evidence.is_empty());
    }

    #[test]
    fn emit_expectation_mismatch_records_expected_tags_and_step_index() {
        let mut event_log = EventLog::new();
        let agent = entity(4);
        let goal_key = GoalKey::from(GoalKind::Sleep);
        let step = PlannedStep {
            def_id: ActionDefId(7),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Craft,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: vec![ExpectedMaterialization {
                hypothetical_id: HypotheticalEntityId(3),
                tag: MaterializationTag::SplitOffLot,
            }],
            guard: None,
            expectations: Vec::new(),
        };

        emit_expectation_mismatch(
            &mut event_log,
            worldwake_core::Tick(12),
            agent,
            goal_key,
            5,
            &step,
            ExpectationMismatchContext {
                mismatch_detail: Some(worldwake_core::MismatchDetail::StateUnmet {
                    predicate: worldwake_core::StatePredicate::ClaimEstablished {
                        claim: worldwake_core::BeliefClaimKey {
                            subject: agent,
                            aspect: worldwake_core::EntityBeliefAspect::Location,
                        },
                    },
                }),
                assumption_refs: AssumptionRefContext::new(
                    &[FrameAssumption::NeedSafeUntilTick {
                        need: HomeostaticNeedId::Fatigue,
                        until_tick: Tick(20),
                    }],
                    5,
                ),
                max_decisive_evidence: 5,
                ..ExpectationMismatchContext::default()
            },
        );

        let events = event_log.events_by_tag(EventTag::ExpectationMismatch);
        assert_eq!(events.len(), 1);
        let payload = event_log
            .get(events[0])
            .and_then(|record| record.decision_payload())
            .expect("expectation mismatch event should carry payload");
        assert_eq!(
            payload,
            &DecisionEventPayload::ExpectationMismatch(
                worldwake_core::ExpectationMismatchPayload {
                    agent,
                    goal_key,
                    step_index: 5,
                    expected_materializations: vec![MaterializationTag::SplitOffLot],
                    expectation_kind: None,
                    mismatch_detail: Some(worldwake_core::MismatchDetail::StateUnmet {
                        predicate: worldwake_core::StatePredicate::ClaimEstablished {
                            claim: worldwake_core::BeliefClaimKey {
                                subject: agent,
                                aspect: worldwake_core::EntityBeliefAspect::Location,
                            },
                        },
                    }),
                    decisive_beliefs: vec![worldwake_core::BeliefRef {
                        claim_key: worldwake_core::BeliefClaimKey {
                            subject: agent,
                            aspect: worldwake_core::EntityBeliefAspect::Location,
                        },
                        claim_held_at_tick: Tick(12),
                        status: worldwake_core::BeliefStatusTag::Contradicted,
                    }],
                    decisive_records: Vec::new(),
                    decisive_world_observations: Vec::new(),
                    assumptions: vec![worldwake_core::PlanAssumptionRef {
                        assumption: FrameAssumption::NeedSafeUntilTick {
                            need: HomeostaticNeedId::Fatigue,
                            until_tick: Tick(20),
                        },
                        introduced_at_step: 0,
                    }],
                }
            )
        );
    }

    #[test]
    fn pending_local_source_reliability_failures_emits_observation_incident() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_resource_source(
                place,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(0),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    quality: None,
                },
            )
            .unwrap();
            commit_txn(txn);
            agent
        };
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let current_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(place),
            },
            goal,
            Vec::new(),
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        )
        .with_committed_source(Some(worldwake_core::SourceKey {
            entity: place,
            commodity: CommodityKind::Apple,
        }))
        .with_expectation_kind(Some(
            OpportunityExpectationKind::AcquireCommodityFromConcreteSource,
        ));

        let view = PerAgentBeliefView::from_world(agent, &world);
        let incidents =
            pending_local_source_reliability_failures(&view, agent, Some(&current_plan), Tick(7));

        assert_eq!(
            incidents,
            vec![OpportunityExpectationFailureIncident {
                opportunity: current_plan.opportunity,
                source: worldwake_core::SourceKey {
                    entity: place,
                    commodity: CommodityKind::Apple,
                },
                expectation_kind: OpportunityExpectationKind::AcquireCommodityFromConcreteSource,
                detected_at_tick: Tick(7),
                phase: ExpectationFailurePhase::Observation,
                cause: ExpectationFailureCause::SourceDepletedLocally,
            }]
        );
    }
}
