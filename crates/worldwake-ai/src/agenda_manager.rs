use crate::{
    AgendaEntry, AgendaEntryKey, AgendaOrigin, AgendaPhase, AgendaState, GoalKind, GoalOffer,
    KillCondition, PartialPlanResumeDecision, PartialPlanResumeTrace, PartialPlanSegment,
    PlanTerminalKind, RevivalTrigger, SkeletonRevalidationContext, SkeletonRevalidationVerdict,
    belief_status::belief_status_tag_for_claim, enterprise::restock_gap_at_destination,
    goal_switching::compare_goal_switch, ranking, revalidate_skeleton_step,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    AgendaProfile, ArtifactLegalEffectTag, AskWitnessMemoryKey, BeliefStatusTag, BlockerKey,
    CommodityKind, Discrepancy, DiscrepancyMemory, EntityId, EntityKind, ExpectationId,
    ExpectationState, GoalKey, IntentionAbandonCondition, IntentionResumeCondition,
    OpportunityAnchor, OpportunityKey, Permille, Quantity, SlotKind, TellTopic, Tick,
};
use worldwake_sim::{GoalBeliefView, RuntimeBeliefView};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgendaTransitions {
    pub killed: Vec<AgendaEntry>,
    pub companion_spawned: Vec<AgendaEntryKey>,
    pub revived: Vec<AgendaEntryKey>,
    pub commit_transition: CommitTransition,
    pub demoted_to_suspended: Vec<AgendaEntryKey>,
    pub demoted_to_pending: Vec<AgendaEntryKey>,
    pub ordered_candidates: Vec<AgendaEntry>,
}

#[derive(Clone, Copy, Debug)]
pub struct AgendaTickPolicy<'a> {
    pub profile: &'a AgendaProfile,
    pub switch_margin: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitTransition {
    Unchanged,
    Committed {
        new_key: AgendaEntryKey,
        previous_key: Option<AgendaEntryKey>,
    },
    Cleared {
        previous_key: AgendaEntryKey,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumedPlan {
    pub key: AgendaEntryKey,
    pub entry: AgendaEntry,
    pub segment: PartialPlanSegment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinationBarrierWatch {
    pub contested_resource: EntityId,
    pub entries: Vec<AgendaEntryKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RejectionLifecycle {
    Satisfied,
    InfeasibleUntil { trigger: RevivalTrigger },
    Dead,
}

pub fn coordination_barrier_watch_list(state: &AgendaState) -> Vec<CoordinationBarrierWatch> {
    let mut watched: BTreeMap<EntityId, Vec<AgendaEntryKey>> = BTreeMap::new();
    for (key, entry) in &state.suspended {
        let Some(segment) = entry.partial_plan_segment.as_ref() else {
            continue;
        };
        let PlanTerminalKind::CoordinationBarrier { contested_resource } = segment.terminal_barrier
        else {
            continue;
        };
        watched.entry(contested_resource).or_default().push(*key);
    }
    watched
        .into_iter()
        .map(|(contested_resource, entries)| CoordinationBarrierWatch {
            contested_resource,
            entries,
        })
        .collect()
}

pub fn try_resume_partial_plan(
    state: &mut AgendaState,
    actor: EntityId,
    belief_view: &dyn RuntimeBeliefView,
    tick: Tick,
    patience_limit: u32,
) -> Option<ResumedPlan> {
    try_resume_partial_plan_with_trace(state, actor, belief_view, tick, patience_limit, None)
}

pub(crate) fn try_resume_partial_plan_with_trace(
    state: &mut AgendaState,
    actor: EntityId,
    belief_view: &dyn RuntimeBeliefView,
    tick: Tick,
    patience_limit: u32,
    mut resume_traces: Option<&mut Vec<PartialPlanResumeTrace>>,
) -> Option<ResumedPlan> {
    let keys: Vec<_> = state.suspended.keys().copied().collect();
    for key in keys {
        let Some(segment) = state
            .suspended
            .get(&key)
            .and_then(|entry| entry.partial_plan_segment.as_ref())
            .cloned()
        else {
            continue;
        };

        if segment.abandon_conditions.iter().any(|condition| {
            partial_plan_abandon_condition_holds(
                condition,
                &segment,
                belief_view,
                actor,
                patience_limit,
            )
        }) {
            record_partial_plan_resume_trace(
                resume_traces.as_deref_mut(),
                &segment,
                PartialPlanResumeDecision::Abandoned,
                Vec::new(),
                None,
            );
            state.suspended.remove(&key);
            remove_companions_for_primary(state, key);
            continue;
        }

        if !segment.resume_conditions.iter().any(|condition| {
            partial_plan_resume_condition_holds(condition, &segment, belief_view, actor, tick)
        }) {
            continue;
        }

        let Some(mut entry) = state.suspended.remove(&key) else {
            continue;
        };
        let mut segment = segment;
        segment.resume_attempt_count = segment.resume_attempt_count.saturating_add(1);
        segment.last_resume_attempt_tick = Some(tick);
        if u32::from(segment.resume_attempt_count) > patience_limit {
            record_partial_plan_resume_trace(
                resume_traces.as_deref_mut(),
                &segment,
                PartialPlanResumeDecision::Abandoned,
                Vec::new(),
                None,
            );
            remove_companions_for_primary(state, key);
            continue;
        }

        let (decision, per_step_verdicts, seeded_ops) =
            classify_partial_plan_resume(actor, belief_view, &segment);
        record_partial_plan_resume_trace(
            resume_traces.as_deref_mut(),
            &segment,
            decision,
            per_step_verdicts,
            seeded_ops,
        );
        if !matches!(decision, PartialPlanResumeDecision::ReusedSeededSearch) {
            segment.remaining_skeleton = None;
        }
        entry.phase = AgendaPhase::Pending;
        entry.last_reconsidered_tick = tick;
        entry.partial_plan_segment = Some(segment.clone());
        return Some(ResumedPlan {
            key,
            entry,
            segment,
        });
    }
    None
}

fn classify_partial_plan_resume(
    actor: EntityId,
    belief_view: &dyn RuntimeBeliefView,
    segment: &PartialPlanSegment,
) -> (
    PartialPlanResumeDecision,
    Vec<SkeletonRevalidationVerdict>,
    Option<Vec<crate::PlannerOpKind>>,
) {
    let Some(skeleton) = segment.remaining_skeleton.as_ref() else {
        return (
            PartialPlanResumeDecision::FallbackToReplanNoSkeleton,
            Vec::new(),
            None,
        );
    };

    let per_step_verdicts = skeleton
        .iter()
        .map(|step| {
            revalidate_skeleton_step(SkeletonRevalidationContext {
                actor,
                goal: &segment.goal,
                step,
                view: belief_view,
            })
        })
        .collect::<Vec<_>>();
    if let Some(reason) = per_step_verdicts.iter().find_map(|verdict| match verdict {
        SkeletonRevalidationVerdict::Reusable => None,
        SkeletonRevalidationVerdict::Invalid(reason) => Some(*reason),
    }) {
        return (
            PartialPlanResumeDecision::FallbackToReplanInvalid(reason),
            per_step_verdicts,
            None,
        );
    }

    (
        PartialPlanResumeDecision::ReusedSeededSearch,
        per_step_verdicts,
        Some(skeleton.iter().map(|step| step.op).collect()),
    )
}

fn record_partial_plan_resume_trace(
    resume_traces: Option<&mut Vec<PartialPlanResumeTrace>>,
    segment: &PartialPlanSegment,
    decision: PartialPlanResumeDecision,
    per_step_verdicts: Vec<SkeletonRevalidationVerdict>,
    seeded_ops: Option<Vec<crate::PlannerOpKind>>,
) {
    if let Some(traces) = resume_traces {
        traces.push(PartialPlanResumeTrace {
            segment_id: segment.id,
            decision,
            per_step_verdicts,
            seeded_ops,
        });
    }
}

pub fn spawn_information_barrier_companions(
    actor: EntityId,
    state: &mut AgendaState,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> Vec<AgendaEntryKey> {
    let keys = state.suspended.keys().copied().collect::<Vec<_>>();
    let mut spawned = Vec::new();
    for primary in keys {
        let Some(entry) = state.suspended.get(&primary) else {
            continue;
        };
        let Some(segment) = entry.partial_plan_segment.as_ref() else {
            continue;
        };
        let PlanTerminalKind::InformationBarrier { topic } = segment.terminal_barrier else {
            continue;
        };
        if companion_for_primary_exists(state, primary, topic) {
            continue;
        }
        let Some(witness) = select_information_barrier_witness(actor, beliefs, topic) else {
            continue;
        };

        let companion = information_barrier_companion_entry(
            entry, primary, witness, topic, actor, beliefs, tick,
        );
        let key = companion.key;
        state.pending.insert(key, companion);
        spawned.push(key);
    }
    spawned
}

fn companion_for_primary_exists(
    state: &AgendaState,
    primary: AgendaEntryKey,
    topic: TellTopic,
) -> bool {
    let matches_companion = |entry: &AgendaEntry| {
        matches!(
            entry.origin,
            AgendaOrigin::Companion {
                primary: owner,
                slot: SlotKind::SocialMotive,
            } if owner == primary
        ) && matches!(
            entry.offer.key.kind,
            GoalKind::AskWitness {
                topic: existing,
                ..
            } if existing == topic
        )
    };

    state.committed.as_ref().is_some_and(matches_companion)
        || state.pending.values().any(matches_companion)
        || state.suspended.values().any(matches_companion)
}

fn information_barrier_companion_entry(
    primary: &AgendaEntry,
    primary_key: AgendaEntryKey,
    witness: EntityId,
    topic: TellTopic,
    actor: EntityId,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> AgendaEntry {
    let goal = GoalKind::AskWitness { witness, topic };
    let anchor = OpportunityAnchor::Entity(witness);
    let mut evidence_entities = BTreeSet::from([witness]);
    if let TellTopic::EntityBelief { subject } = topic {
        evidence_entities.insert(subject);
    }
    let evidence_places = beliefs
        .effective_place(actor)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let offer = GoalOffer {
        key: GoalKey::from(goal),
        anchor,
        evidence_entities,
        evidence_places,
        obligation_source: None,
        commitment_impact_if_ignored: primary.offer.commitment_impact_if_ignored,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: primary.offer.motive_sources.clone(),
        acquisition_quantity: None,
    };

    AgendaEntry {
        key: OpportunityKey {
            goal_key: offer.key,
            anchor: offer.anchor,
        },
        offer,
        phase: AgendaPhase::Pending,
        origin: AgendaOrigin::Companion {
            primary: primary_key,
            slot: SlotKind::SocialMotive,
        },
        introduced_tick: tick,
        last_reconsidered_tick: tick,
        revival_trigger: None,
        kill_condition: KillCondition::External,
        priority_class: primary.priority_class,
        motive_score: primary.motive_score.max(1),
        motive_source_contributions: primary.motive_source_contributions.clone(),
        provenance: primary.provenance.clone(),
        source_reliability_discount: primary.source_reliability_discount.clone(),
        competition_discount: primary.competition_discount.clone(),
        source_composite: primary.source_composite,
        feasibility: primary.feasibility,
        partial_plan_segment: None,
    }
}

pub(crate) fn select_information_barrier_witness(
    actor: EntityId,
    beliefs: &impl GoalBeliefView,
    topic: TellTopic,
) -> Option<EntityId> {
    let TellTopic::EntityBelief { subject } = topic else {
        return None;
    };
    let place = beliefs.effective_place(actor)?;
    beliefs
        .entities_at(place)
        .into_iter()
        .filter(|candidate| *candidate != actor)
        .filter(|candidate| beliefs.entity_kind(*candidate) == Some(EntityKind::Agent))
        .filter(|candidate| beliefs.is_alive(*candidate) && !beliefs.is_incapacitated(*candidate))
        .filter(|candidate| {
            let key = AskWitnessMemoryKey {
                counterparty: *candidate,
                topic_entity: Some(subject),
                topic_commodity: None,
            };
            beliefs.ask_witness_memory(actor, &key).is_none()
        })
        .filter(|candidate| witness_plausibly_knows_topic(actor, *candidate, subject, beliefs))
        .min()
}

fn witness_plausibly_knows_topic(
    actor: EntityId,
    witness: EntityId,
    subject: EntityId,
    beliefs: &impl GoalBeliefView,
) -> bool {
    if beliefs
        .entity_beliefs_sourced_from_witness(actor, witness)
        .into_iter()
        .any(|(entity, _)| entity == subject)
    {
        return true;
    }

    beliefs
        .effective_place(subject)
        .is_some_and(|place| beliefs.effective_place(witness) == Some(place))
}

fn remove_companions_for_primary(state: &mut AgendaState, primary: AgendaEntryKey) {
    let is_owned_companion = |entry: &AgendaEntry| {
        matches!(
            entry.origin,
            AgendaOrigin::Companion {
                primary: owner,
                slot: SlotKind::SocialMotive,
            } if owner == primary
        )
    };

    if state.committed.as_ref().is_some_and(is_owned_companion) {
        state.committed = None;
    }
    state.pending.retain(|_, entry| !is_owned_companion(entry));
    state
        .suspended
        .retain(|_, entry| !is_owned_companion(entry));
}

fn partial_plan_abandon_condition_holds(
    condition: &IntentionAbandonCondition,
    segment: &PartialPlanSegment,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    patience_limit: u32,
) -> bool {
    match condition {
        IntentionAbandonCondition::MotiveSourceLost(_)
        | IntentionAbandonCondition::AssumptionPermanentlyBroken(_) => false,
        IntentionAbandonCondition::OpportunityForeverGone(anchor) => {
            opportunity_anchor_gone(*anchor, view)
        }
        IntentionAbandonCondition::PatienceExhausted => {
            u32::from(segment.resume_attempt_count) >= patience_limit
        }
        IntentionAbandonCondition::ArtifactDestroyed(artifact) => {
            known_artifact_state(view, agent, *artifact).is_some_and(|artifact_state| {
                matches!(
                    artifact_state.existence,
                    worldwake_core::ArtifactExistence::Destroyed { .. }
                )
            })
        }
        IntentionAbandonCondition::ArtifactLegalEffectLost(artifact) => {
            known_artifact_state(view, agent, *artifact).is_some_and(|artifact_state| {
                ArtifactLegalEffectTag::from(&artifact_state.legal_effect)
                    != ArtifactLegalEffectTag::Active
            })
        }
    }
}

fn partial_plan_resume_condition_holds(
    condition: &IntentionResumeCondition,
    segment: &PartialPlanSegment,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    tick: Tick,
) -> bool {
    match condition {
        IntentionResumeCondition::BeliefStatusChanged {
            subject,
            target_status,
        } => belief_status_matches(view, agent, *subject, *target_status, tick),
        IntentionResumeCondition::OpportunityVisible(anchor) => {
            opportunity_anchor_visible(*anchor, view, agent)
        }
        IntentionResumeCondition::LocationReached(place) => {
            view.effective_place(agent) == Some(*place)
        }
        IntentionResumeCondition::TickElapsed(elapsed) => {
            tick.0.saturating_sub(segment.created_tick.0) >= u64::from(*elapsed)
        }
        IntentionResumeCondition::ArtifactLegalEffectActive(artifact) => {
            coordination_barrier_resource_available(segment, view, agent, *artifact)
                || known_artifact_state(view, agent, *artifact).is_some_and(|artifact_state| {
                    ArtifactLegalEffectTag::from(&artifact_state.legal_effect)
                        == ArtifactLegalEffectTag::Active
                })
        }
    }
}

fn coordination_barrier_resource_available(
    segment: &PartialPlanSegment,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    artifact: EntityId,
) -> bool {
    let PlanTerminalKind::CoordinationBarrier { contested_resource } = segment.terminal_barrier
    else {
        return false;
    };
    if contested_resource != artifact {
        return false;
    }

    if view.has_contention_policy(contested_resource) {
        return view
            .facility_grant(contested_resource)
            .is_none_or(|grant| grant.actor == agent);
    }

    view.has_extraction_queues(contested_resource)
        && (view.actor_holds_extraction_slot_grant(contested_resource, agent)
            || view.actor_can_claim_extraction_slot(contested_resource, agent))
}

fn opportunity_anchor_gone(anchor: OpportunityAnchor, view: &dyn RuntimeBeliefView) -> bool {
    match anchor {
        OpportunityAnchor::Place(place) | OpportunityAnchor::Entity(place)
            if view.entity_kind(place).is_none() =>
        {
            true
        }
        OpportunityAnchor::None | OpportunityAnchor::Place(_) => false,
        OpportunityAnchor::Entity(entity) => view.is_dead(entity),
    }
}

fn opportunity_anchor_visible(
    anchor: OpportunityAnchor,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> bool {
    match anchor {
        OpportunityAnchor::None => true,
        OpportunityAnchor::Place(place) => view.effective_place(agent) == Some(place),
        OpportunityAnchor::Entity(entity) => view
            .effective_place(entity)
            .is_some_and(|place| view.effective_place(agent) == Some(place)),
    }
}

fn known_artifact_state(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    artifact: EntityId,
) -> Option<worldwake_core::BelievedArtifactState> {
    view.known_entity_beliefs(agent)
        .into_iter()
        .find(|(entity, _)| *entity == artifact)
        .and_then(|(_, state)| state.believed_artifact)
}

fn belief_status_matches(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    subject: EntityId,
    target_status: BeliefStatusTag,
    tick: Tick,
) -> bool {
    let Some(store) = view.agent_belief_store(agent) else {
        return false;
    };
    let Some(claims) = store.entity_claims.get(&subject) else {
        return false;
    };

    if target_status == BeliefStatusTag::Disputed {
        return claims
            .iter()
            .filter(|claim| claim.refuted_at_tick.is_none())
            .take(2)
            .count()
            > 1;
    }

    claims
        .iter()
        .any(|claim| belief_status_tag_for_claim(view, agent, claim, tick) == target_status)
}

pub(crate) fn classify_rejection(
    actor: EntityId,
    probe_verdict: &crate::agent_tick::portfolio::FeasibilityVerdict,
    offer: &GoalOffer,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
    revive_cooldown_ticks: u32,
) -> RejectionLifecycle {
    if goal_post_conditions_already_satisfied(actor, offer, beliefs) {
        return RejectionLifecycle::Satisfied;
    }

    let reason = match probe_verdict {
        crate::agent_tick::portfolio::FeasibilityVerdict::RejectedBeforeSearch { reason } => {
            *reason
        }
        crate::agent_tick::portfolio::FeasibilityVerdict::Plausible => {
            unreachable!("classify_rejection only accepts rejected probe verdicts")
        }
    };

    match reason {
        Discrepancy::MissingObservation | Discrepancy::Omission(_) => {
            let place = offer_anchor_place(offer).or_else(|| {
                offer_anchor_entity(offer).and_then(|entity| beliefs.effective_place(entity))
            });
            if let Some(kind) = offer_commodity(offer) {
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::CommodityAvailable {
                        place: place.expect("commodity revival trigger needs a place"),
                        kind,
                        min: Quantity(1),
                    },
                }
            } else {
                let target = offer_anchor_entity(offer)
                    .or(offer.key.entity)
                    .expect("target-present revival trigger needs a target");
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::TargetPresent {
                        target,
                        place: place
                            .or_else(|| beliefs.effective_place(target))
                            .expect("target-present revival trigger needs a place"),
                    },
                }
            }
        }
        Discrepancy::RouteUnknown => RejectionLifecycle::InfeasibleUntil {
            trigger: RevivalTrigger::RouteLearned {
                from: beliefs
                    .effective_place(actor)
                    .expect("route-learned revival trigger needs actor place"),
                to: offer_anchor_place(offer)
                    .or_else(|| {
                        offer_anchor_entity(offer)
                            .and_then(|entity| beliefs.effective_place(entity))
                    })
                    .expect("route-learned revival trigger needs destination place"),
            },
        },
        Discrepancy::PartialExecutionDrift
        | Discrepancy::BeliefStale
        | Discrepancy::SourceInvalidated
        | Discrepancy::SearchBudgetExhausted
        | Discrepancy::NeedHorizonExceeded { .. }
        | Discrepancy::ArtifactNotActionable { .. }
        | Discrepancy::AbandonConditionFired(_)
        | Discrepancy::MethodFailure(_) => RejectionLifecycle::InfeasibleUntil {
            trigger: RevivalTrigger::TickElapsed {
                at_tick: Tick(tick.0.saturating_add(u64::from(revive_cooldown_ticks))),
            },
        },
        Discrepancy::NoWillingCounterparty => {
            let counterparty = offer_anchor_entity(offer)
                .or(offer.key.entity)
                .expect("counterparty revival trigger needs counterparty entity");
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CounterpartyAvailable {
                    counterparty,
                    place: offer_anchor_place(offer)
                        .or_else(|| beliefs.effective_place(counterparty))
                        .expect("counterparty revival trigger needs a place"),
                },
            }
        }
        Discrepancy::BeliefContradicted
        | Discrepancy::NoLegalBinding
        | Discrepancy::ImproperPlanningState => RejectionLifecycle::Dead,
    }
}

pub fn tick_agenda(
    actor: EntityId,
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    policy: AgendaTickPolicy<'_>,
    tick: Tick,
) -> AgendaTransitions {
    let previous_committed = state
        .committed
        .as_ref()
        .filter(|entry| entry.phase == AgendaPhase::Committed)
        .map(|entry| entry.key);
    let killed = drain_killed(actor, state, beliefs, tick);
    for entry in &killed {
        remove_companions_for_primary(state, entry.key);
    }
    let companion_spawned = spawn_information_barrier_companions(actor, state, beliefs, tick);
    let revived_entries = promote_revived(
        actor,
        state,
        beliefs,
        discrepancy_memory,
        policy.profile,
        tick,
    );
    let revived = revived_entries.iter().map(|entry| entry.key).collect();
    let merged = merge_candidates(state, fresh_candidates, tick);
    let ranked = rank_for_commit(state, merged, revived_entries);
    let commit_transition = commit_or_keep(
        state,
        ranked.as_slice(),
        previous_committed,
        policy.switch_margin,
        tick,
    );
    let ordered_candidates = ranked.clone();
    let (demoted_to_pending, demoted_to_suspended) =
        demote_to_pending_or_suspended(state, ranked, policy.profile, tick);

    AgendaTransitions {
        killed,
        companion_spawned,
        revived,
        commit_transition,
        demoted_to_suspended,
        demoted_to_pending,
        ordered_candidates,
    }
}

fn drain_killed(
    actor: EntityId,
    state: &mut AgendaState,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut killed = Vec::new();

    if state
        .committed
        .as_ref()
        .is_some_and(|entry| should_kill(actor, &entry.kill_condition, beliefs, tick))
        && let Some(entry) = state.committed.take()
    {
        killed.push(entry);
    }

    drain_killed_map(actor, &mut state.pending, beliefs, tick, &mut killed);
    drain_killed_map(actor, &mut state.suspended, beliefs, tick, &mut killed);
    killed
}

fn drain_killed_map(
    actor: EntityId,
    entries: &mut BTreeMap<AgendaEntryKey, AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
    killed: &mut Vec<AgendaEntry>,
) {
    let keys: Vec<_> = entries
        .iter()
        .filter_map(|(key, entry)| {
            should_kill(actor, &entry.kill_condition, beliefs, tick).then_some(*key)
        })
        .collect();
    for key in keys {
        if let Some(entry) = entries.remove(&key) {
            killed.push(entry);
        }
    }
}

fn promote_revived(
    actor: EntityId,
    state: &mut AgendaState,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    profile: &AgendaProfile,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut revived = Vec::new();
    let keys: Vec<_> = state.pending.keys().copied().collect();

    for key in keys {
        let Some(entry) = state.pending.get(&key) else {
            continue;
        };
        let cooldown_ready = tick.0
            >= entry
                .last_reconsidered_tick
                .0
                .saturating_add(u64::from(profile.revive_cooldown_ticks));
        if !cooldown_ready
            || discrepancy_memory.is_suppressed(&blocker_key_from(entry).into(), tick)
        {
            continue;
        }
        let Some(trigger) = entry.revival_trigger.as_ref() else {
            continue;
        };
        if !evaluate_revival_trigger(actor, trigger, beliefs, tick) {
            continue;
        }

        let mut revived_entry = state
            .pending
            .remove(&key)
            .expect("pending key disappeared during revival pass");
        revived_entry.last_reconsidered_tick = tick;
        revived_entry.phase = AgendaPhase::Pending;
        let revived_goal = revived_entry.key.goal_key;
        let revived_trigger = revived_entry.revival_trigger.clone();
        state.pending.retain(|_, pending| {
            pending.key.goal_key != revived_goal || pending.revival_trigger != revived_trigger
        });
        revived.push(revived_entry);
    }

    revived
}

fn merge_candidates(
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut deduped = BTreeMap::new();
    for candidate in fresh_candidates {
        deduped.insert(candidate.key, candidate);
    }

    let mut merged = Vec::with_capacity(deduped.len());
    for (key, candidate) in deduped {
        if let Some(committed) = state.committed.as_mut()
            && committed.key == key
        {
            refresh_entry(committed, &candidate, tick);
            committed.phase = AgendaPhase::Committed;
            continue;
        }
        if state
            .committed
            .as_ref()
            .is_some_and(|committed| committed.key.goal_key == key.goal_key)
        {
            continue;
        }

        if let Some(mut existing) = state.pending.remove(&key) {
            refresh_entry(&mut existing, &candidate, tick);
            existing.phase = AgendaPhase::Pending;
            merged.push(existing);
            continue;
        }

        if let Some(mut existing) = state.suspended.remove(&key) {
            refresh_entry(&mut existing, &candidate, tick);
            if existing.partial_plan_segment.is_some() {
                existing.phase = AgendaPhase::Suspended;
                state.suspended.insert(key, existing);
                continue;
            }
            existing.phase = AgendaPhase::Pending;
            merged.push(existing);
            continue;
        }

        let mut candidate = candidate;
        candidate.phase = AgendaPhase::Pending;
        candidate.last_reconsidered_tick = tick;
        merged.push(candidate);
    }

    merged
}

fn refresh_entry(existing: &mut AgendaEntry, candidate: &AgendaEntry, tick: Tick) {
    existing.offer = candidate.offer.clone();
    existing.priority_class = candidate.priority_class;
    existing.motive_score = candidate.motive_score;
    existing.provenance.clone_from(&candidate.provenance);
    existing
        .source_reliability_discount
        .clone_from(&candidate.source_reliability_discount);
    existing
        .competition_discount
        .clone_from(&candidate.competition_discount);
    existing.feasibility = candidate.feasibility;
    existing.last_reconsidered_tick = tick;
}

fn rank_for_commit(
    state: &AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    revived: Vec<AgendaEntry>,
) -> Vec<AgendaEntry> {
    let mut pool = Vec::new();
    if let Some(committed) = state.committed.clone() {
        pool.push(committed);
    }
    pool.extend(revived);
    pool.extend(fresh_candidates);
    let _ = ranking::sort_in_place(&mut pool);
    pool
}

fn commit_or_keep(
    state: &mut AgendaState,
    ranked: &[AgendaEntry],
    previous_committed: Option<AgendaEntryKey>,
    switch_margin: Permille,
    tick: Tick,
) -> CommitTransition {
    let Some(top) = ranked.first() else {
        state.committed = None;
        return previous_committed.map_or(CommitTransition::Unchanged, |previous_key| {
            CommitTransition::Cleared { previous_key }
        });
    };

    let Some(current) = state.committed.as_ref() else {
        state.committed = Some(AgendaEntry::committed_from(top, tick));
        return CommitTransition::Committed {
            new_key: top.key,
            previous_key: previous_committed,
        };
    };

    if current.phase != AgendaPhase::Committed {
        return CommitTransition::Unchanged;
    }

    if current.key == top.key {
        let mut refreshed = top.clone();
        refreshed.phase = AgendaPhase::Committed;
        refreshed.introduced_tick = current.introduced_tick;
        refreshed.last_reconsidered_tick = tick;
        state.committed = Some(refreshed);
        return CommitTransition::Unchanged;
    }

    let should_switch = compare_goal_switch(
        current.priority_class,
        Some(current.motive_score),
        top.priority_class,
        top.motive_score,
        switch_margin,
    )
    .is_some();
    if !should_switch {
        return CommitTransition::Unchanged;
    }

    state.committed = Some(AgendaEntry::committed_from(top, tick));
    CommitTransition::Committed {
        new_key: top.key,
        previous_key: previous_committed,
    }
}

fn demote_to_pending_or_suspended(
    state: &mut AgendaState,
    ranked: Vec<AgendaEntry>,
    profile: &AgendaProfile,
    tick: Tick,
) -> (Vec<AgendaEntryKey>, Vec<AgendaEntryKey>) {
    let committed_key = state.committed.as_ref().map(|entry| entry.key);
    let committed_goal = state.committed.as_ref().map(|entry| entry.key.goal_key);
    let mut demoted_to_pending = Vec::new();
    let demoted_to_suspended = Vec::new();
    let mut seen = BTreeSet::new();

    for mut entry in ranked {
        if Some(entry.key) == committed_key
            || Some(entry.key.goal_key) == committed_goal
            || !seen.insert(entry.key)
        {
            continue;
        }
        if entry.revival_trigger.is_none() && entry.introduced_tick == tick {
            continue;
        }
        entry.phase = AgendaPhase::Pending;
        entry.last_reconsidered_tick = tick;
        if entry.revival_trigger.is_none() {
            entry.revival_trigger = Some(RevivalTrigger::TickElapsed {
                at_tick: Tick(
                    tick.0
                        .saturating_add(u64::from(profile.revive_cooldown_ticks.max(1))),
                ),
            });
        }
        let key = entry.key;
        insert_with_capacity(&mut state.pending, profile.pending_capacity, entry);
        demoted_to_pending.push(key);
    }

    (demoted_to_pending, demoted_to_suspended)
}

fn insert_with_capacity(
    entries: &mut BTreeMap<AgendaEntryKey, AgendaEntry>,
    capacity: u32,
    entry: AgendaEntry,
) {
    entries.insert(entry.key, entry);
    let capacity = usize::try_from(capacity).unwrap_or(usize::MAX);
    while entries.len() > capacity {
        let evict_key = entries
            .iter()
            .min_by_key(|(key, entry)| (entry.last_reconsidered_tick, **key))
            .map(|(key, _)| *key)
            .expect("entries must be non-empty when enforcing capacity");
        entries.remove(&evict_key);
    }
}

fn evaluate_revival_trigger(
    actor: EntityId,
    trigger: &RevivalTrigger,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> bool {
    match trigger {
        RevivalTrigger::CommodityAvailable { place, kind, min } => {
            beliefs.controlled_commodity_quantity_at_place(actor, *place, *kind) >= *min
        }
        RevivalTrigger::TargetPresent { target, place } => {
            beliefs.effective_place(*target) == Some(*place) && beliefs.is_alive(*target)
        }
        RevivalTrigger::RouteLearned { from, to } => {
            *from == *to
                || beliefs
                    .adjacent_places_with_travel_ticks(*from)
                    .iter()
                    .any(|(neighbor, _)| neighbor == to)
        }
        RevivalTrigger::CounterpartyAvailable {
            counterparty,
            place,
        } => {
            beliefs.effective_place(*counterparty) == Some(*place)
                && beliefs.is_alive(*counterparty)
                && !beliefs.is_incapacitated(*counterparty)
        }
        RevivalTrigger::TickElapsed { at_tick } => tick >= *at_tick,
    }
}

fn should_kill(
    actor: EntityId,
    condition: &KillCondition,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> bool {
    match condition {
        KillCondition::TickExpiry { at_tick } => tick >= *at_tick,
        KillCondition::ObligationResolved { expectation } => {
            expectation_is_resolved(actor, *expectation, beliefs)
        }
        KillCondition::TargetDead { target } => {
            beliefs.is_dead(*target) || !beliefs.is_alive(*target)
        }
        KillCondition::External => false,
    }
}

fn expectation_is_resolved(
    actor: EntityId,
    expectation: ExpectationId,
    beliefs: &impl GoalBeliefView,
) -> bool {
    beliefs
        .expectation_store(actor)
        .and_then(|store| store.records.get(&expectation).copied())
        .is_none_or(|record| {
            !matches!(
                record.state,
                ExpectationState::Active | ExpectationState::Overdue
            )
        })
}

pub(crate) fn goal_post_conditions_already_satisfied(
    actor: EntityId,
    offer: &GoalOffer,
    beliefs: &impl GoalBeliefView,
) -> bool {
    match offer.key.kind {
        GoalKind::MoveCargo {
            commodity,
            destination,
        } => {
            let destination_place = beliefs.effective_place(destination).unwrap_or(destination);
            (beliefs.effective_place(actor) == Some(destination_place)
                && beliefs.commodity_quantity(actor, commodity) > Quantity(0))
                || restock_gap_at_destination(beliefs, actor, destination, commodity).is_none()
        }
        _ => false,
    }
}

fn offer_commodity(offer: &GoalOffer) -> Option<CommodityKind> {
    match offer.key.kind {
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::RestockCommodity { commodity }
        | GoalKind::MoveCargo { commodity, .. }
        | GoalKind::SellCommodity { commodity } => Some(commodity),
        _ => None,
    }
}

fn offer_anchor_place(offer: &GoalOffer) -> Option<EntityId> {
    anchor_place(offer.anchor).or(offer.key.place)
}

fn offer_anchor_entity(offer: &GoalOffer) -> Option<EntityId> {
    anchor_entity(offer.anchor).or(offer.key.entity)
}

fn blocker_key_from(entry: &AgendaEntry) -> BlockerKey {
    BlockerKey {
        goal_key: entry.offer.key,
        place: anchor_place(entry.offer.anchor).or(entry.offer.key.place),
        target: anchor_entity(entry.offer.anchor).or(entry.offer.key.entity),
        action_def: None,
    }
}

fn anchor_place(anchor: OpportunityAnchor) -> Option<EntityId> {
    match anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => None,
    }
}

fn anchor_entity(anchor: OpportunityAnchor) -> Option<EntityId> {
    match anchor {
        OpportunityAnchor::Entity(entity) => Some(entity),
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgendaTickPolicy, AgendaTransitions, CommitTransition, RejectionLifecycle,
        classify_rejection, tick_agenda,
    };
    use crate::agent_tick::portfolio::FeasibilityVerdict;
    use crate::{
        AgendaEntry, AgendaOrigin, AgendaPhase, AgendaState, BarrierFact, FeasibilityHint, GoalKey,
        GoalKind, GoalOffer, GoalPriorityClass, KillCondition, PartialPlanResumeDecision,
        PartialPlanSegment, PartialPlanSegmentId, PlanTerminalKind, PlannedSkeletonStep,
        PlannerOpKind, RevivalTrigger, SkeletonRevalidationReason, SkeletonRevalidationVerdict,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, AgendaProfile, AskWitnessMemory, AskWitnessMemoryKey,
        BeliefConfidencePolicy, BelievedEntityState, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DemandObservationReason, Discrepancy, DiscrepancyMemory,
        DriveThresholds, EntityId, EntityKind, ExpectationBasis, ExpectationId, ExpectationRecord,
        ExpectationState, ExpectationStore, IntentionAbandonCondition, IntentionResumeCondition,
        LoadUnits, MerchandiseProfile, OpportunityAnchor, OpportunityKey, PerceptionSource,
        Quantity, ResourceSource, SlotKind, TellTopic, Tick, UniqueItemKind, WorkstationTag,
    };
    use worldwake_sim::{
        ActionDuration, CombatBeliefView, ControlBeliefView, DurationExpr, EconomicBeliefView,
        EntityBeliefView, FacilityBeliefView, GoalBeliefView, InventoryBeliefView,
        ProfileBeliefView, RuntimeBeliefView, SpatialBeliefView, TemporalBeliefView,
    };

    const AGENT: EntityId = EntityId {
        slot: 1,
        generation: 0,
    };
    const PLACE: EntityId = EntityId {
        slot: 2,
        generation: 0,
    };
    const TARGET: EntityId = EntityId {
        slot: 3,
        generation: 0,
    };
    const CONTESTED_RESOURCE: EntityId = EntityId {
        slot: 4,
        generation: 0,
    };

    #[derive(Clone, Default)]
    struct MockGoalBeliefView {
        controlled_quantities: BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
        entity_places: BTreeMap<EntityId, EntityId>,
        entities_by_place: BTreeMap<EntityId, Vec<EntityId>>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        known_entity_beliefs: BTreeMap<EntityId, BelievedEntityState>,
        dead: BTreeSet<EntityId>,
        incapacitated: BTreeSet<EntityId>,
        ask_witness_memory: BTreeMap<AskWitnessMemoryKey, AskWitnessMemory>,
        adjacency: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        expectation_store: Option<ExpectationStore>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
    }

    impl worldwake_sim::BelievedAuthorityView for MockGoalBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for MockGoalBeliefView {}

    impl GoalBeliefView for MockGoalBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            !self.dead.contains(&entity)
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.entity_places.get(&entity).copied()
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_by_place
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }

        fn known_entity_beliefs(&self, _agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.known_entity_beliefs
                .iter()
                .map(|(entity, belief)| (*entity, belief.clone()))
                .collect()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacency.get(&place).cloned().unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            agent: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.controlled_quantities
                .get(&(agent, place, commodity))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn is_incapacitated(&self, entity: EntityId) -> bool {
            self.incapacitated.contains(&entity)
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<worldwake_core::HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn expectation_store(&self, _agent: EntityId) -> Option<ExpectationStore> {
            self.expectation_store.clone()
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<worldwake_core::DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn ask_witness_memory(
            &self,
            _actor: EntityId,
            key: &AskWitnessMemoryKey,
        ) -> Option<AskWitnessMemory> {
            self.ask_witness_memory.get(key).cloned()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    #[derive(Clone, Default)]
    struct ResumeBeliefView {
        entity_places: BTreeMap<EntityId, EntityId>,
        dead: BTreeSet<EntityId>,
        contention_policies: BTreeSet<EntityId>,
        facility_grants: BTreeMap<EntityId, worldwake_core::ContentionGrant>,
        extraction_sources: BTreeSet<EntityId>,
        extraction_grants: BTreeSet<(EntityId, EntityId)>,
        extraction_claimable: BTreeSet<(EntityId, EntityId)>,
    }

    impl worldwake_sim::BelievedAuthorityView for ResumeBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for ResumeBeliefView {}
    impl RuntimeBeliefView for ResumeBeliefView {}

    impl ControlBeliefView for ResumeBeliefView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EntityBeliefView for ResumeBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            !self.dead.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            (!self.dead.contains(&entity) && self.entity_places.contains_key(&entity))
                .then_some(EntityKind::Agent)
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for ResumeBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<worldwake_core::HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MetabolismProfile> {
            None
        }
    }

    impl SpatialBeliefView for ResumeBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.entity_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn route_exists(&self, from: EntityId, to: EntityId) -> bool {
            from == to
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<worldwake_core::InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
    }

    impl TemporalBeliefView for ResumeBeliefView {
        fn has_contention_policy(&self, entity: EntityId) -> bool {
            self.contention_policies.contains(&entity)
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&worldwake_core::ContentionGrant> {
            self.facility_grants.get(&facility)
        }

        fn actor_holds_extraction_slot_grant(&self, source: EntityId, actor: EntityId) -> bool {
            self.extraction_grants.contains(&(source, actor))
        }

        fn actor_can_claim_extraction_slot(&self, source: EntityId, actor: EntityId) -> bool {
            self.extraction_claimable.contains(&(source, actor))
        }

        fn has_extraction_queues(&self, source: EntityId) -> bool {
            self.extraction_sources.contains(&source)
        }

        fn reservation_conflicts(
            &self,
            _entity: EntityId,
            _range: worldwake_core::TickRange,
        ) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<worldwake_core::TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &worldwake_sim::ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    impl InventoryBeliefView for ResumeBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }
    }

    impl CombatBeliefView for ResumeBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EconomicBeliefView for ResumeBeliefView {
        fn trade_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TradeDispositionProfile> {
            None
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl worldwake_sim::SocialBeliefView for ResumeBeliefView {
        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
    }

    impl worldwake_sim::PoliticalBeliefView for ResumeBeliefView {}

    impl FacilityBeliefView for ResumeBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn goal_offer(goal: GoalKind, anchor: OpportunityAnchor) -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(goal),
            anchor,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        }
    }

    fn agenda_entry(
        goal: GoalKind,
        anchor: OpportunityAnchor,
        motive_score: u32,
        introduced_tick: Tick,
    ) -> AgendaEntry {
        AgendaEntry {
            key: OpportunityKey {
                goal_key: GoalKey::from(goal),
                anchor,
            },
            offer: goal_offer(goal, anchor),
            phase: AgendaPhase::Pending,
            origin: AgendaOrigin::NeedDrive,
            introduced_tick,
            last_reconsidered_tick: introduced_tick,
            revival_trigger: None,
            kill_condition: KillCondition::External,
            priority_class: GoalPriorityClass::Medium,
            motive_score,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,
            partial_plan_segment: None,
        }
    }

    fn partial_plan_segment(
        resume_conditions: Vec<IntentionResumeCondition>,
        abandon_conditions: Vec<IntentionAbandonCondition>,
        resume_attempt_count: u8,
    ) -> PartialPlanSegment {
        PartialPlanSegment {
            id: PartialPlanSegmentId::new(Tick(10), resume_attempt_count.into()),
            goal: goal_offer(GoalKind::Sleep, OpportunityAnchor::Place(PLACE)),
            completed_prefix: Vec::new(),
            remaining_skeleton: None,
            terminal_barrier: PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 3,
                budget_total: 3,
            },
            barrier_fact: BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            },
            resume_conditions,
            abandon_conditions,
            created_tick: Tick(10),
            last_resume_attempt_tick: None,
            resume_attempt_count,
            causal_links: Vec::new(),
        }
    }

    fn skeleton_step(expected_pre: Vec<crate::htn::BeliefPredicate>) -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::Sleep,
            target_template: crate::htn::PayloadTemplate::FromContext,
            expected_pre,
        }
    }

    fn information_barrier_segment(subject: EntityId) -> PartialPlanSegment {
        PartialPlanSegment {
            terminal_barrier: PlanTerminalKind::InformationBarrier {
                topic: TellTopic::EntityBelief { subject },
            },
            barrier_fact: BarrierFact::MissingBelief(
                crate::htn::BeliefPredicate::TargetLastSeenKnown {
                    target: crate::htn::EntityTemplate::Fixed(subject),
                },
            ),
            ..partial_plan_segment(
                vec![IntentionResumeCondition::BeliefStatusChanged {
                    subject,
                    target_status: worldwake_core::BeliefStatusTag::Certain,
                }],
                vec![IntentionAbandonCondition::PatienceExhausted],
                0,
            )
        }
    }

    fn coordination_barrier_segment(resource: EntityId) -> PartialPlanSegment {
        PartialPlanSegment {
            terminal_barrier: PlanTerminalKind::CoordinationBarrier {
                contested_resource: resource,
            },
            barrier_fact: BarrierFact::ContestedReservation(resource),
            resume_conditions: vec![IntentionResumeCondition::ArtifactLegalEffectActive(
                resource,
            )],
            ..partial_plan_segment(
                Vec::new(),
                vec![IntentionAbandonCondition::PatienceExhausted],
                0,
            )
        }
    }

    fn suspended_partial_entry(segment: PartialPlanSegment) -> AgendaEntry {
        let mut entry = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            50,
            Tick(10),
        );
        entry.phase = AgendaPhase::Suspended;
        entry.partial_plan_segment = Some(segment);
        entry
    }

    fn reported_belief_from(witness: EntityId) -> BelievedEntityState {
        BelievedEntityState {
            source: PerceptionSource::Report {
                from: witness,
                chain_len: 0,
            },
            ..BelievedEntityState::single_observation_defaults(
                Tick(9),
                PerceptionSource::Report {
                    from: witness,
                    chain_len: 0,
                },
            )
        }
    }

    fn information_barrier_beliefs(subject: EntityId, witness: EntityId) -> MockGoalBeliefView {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(AGENT, PLACE);
        beliefs.entity_places.insert(witness, PLACE);
        beliefs.entity_places.insert(subject, PLACE);
        beliefs
            .entities_by_place
            .insert(PLACE, vec![AGENT, witness]);
        beliefs.entity_kinds.insert(AGENT, EntityKind::Agent);
        beliefs.entity_kinds.insert(witness, EntityKind::Agent);
        beliefs
            .known_entity_beliefs
            .insert(subject, reported_belief_from(witness));
        beliefs
    }

    fn default_profile() -> AgendaProfile {
        AgendaProfile {
            pending_capacity: 16,
            suspended_capacity: 8,
            revive_cooldown_ticks: 4,
        }
    }

    fn demand(place: EntityId, commodity: CommodityKind, quantity: u32) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(quantity),
            place,
            tick: Tick(1),
            counterparty: None,
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn default_switch_margin() -> worldwake_core::Permille {
        worldwake_core::Permille::new(100).unwrap()
    }

    fn contention_grant(actor: EntityId) -> worldwake_core::ContentionGrant {
        worldwake_core::ContentionGrant {
            actor,
            intended_action: worldwake_core::ActionDefId(99),
            granted_at: Tick(12),
            expires_at: Tick(16),
        }
    }

    fn classify(
        goal: GoalKind,
        anchor: OpportunityAnchor,
        reason: Discrepancy,
    ) -> RejectionLifecycle {
        classify_rejection(
            AGENT,
            &FeasibilityVerdict::RejectedBeforeSearch { reason },
            &goal_offer(goal, anchor),
            &MockGoalBeliefView::default(),
            Tick(10),
            default_profile().revive_cooldown_ticks,
        )
    }

    #[test]
    fn try_resume_partial_plan_returns_segment_when_resume_condition_holds() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            1,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));

        let resumed = super::try_resume_partial_plan(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
        )
        .expect("eligible partial segment should resume");

        assert_eq!(resumed.key, key);
        assert_eq!(resumed.entry.phase, AgendaPhase::Pending);
        assert_eq!(resumed.segment.resume_attempt_count, 2);
        assert_eq!(resumed.segment.last_resume_attempt_tick, Some(Tick(14)));
        assert!(state.suspended.is_empty());
    }

    #[test]
    fn try_resume_with_reusable_skeleton_emits_reuse_trace_and_keeps_seed() {
        let mut segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            0,
        );
        segment.remaining_skeleton = Some(vec![skeleton_step(Vec::new())]);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment.clone()));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
            Some(&mut traces),
        )
        .expect("eligible partial segment should resume");

        assert_eq!(
            resumed
                .entry
                .partial_plan_segment
                .as_ref()
                .and_then(|segment| segment.remaining_skeleton.as_ref()),
            segment.remaining_skeleton.as_ref()
        );
        assert_eq!(traces.len(), 1);
        assert_eq!(
            traces[0].decision,
            PartialPlanResumeDecision::ReusedSeededSearch
        );
        assert_eq!(
            traces[0].per_step_verdicts,
            vec![SkeletonRevalidationVerdict::Reusable]
        );
        assert_eq!(traces[0].seeded_ops, Some(vec![PlannerOpKind::Sleep]));
    }

    #[test]
    fn try_resume_with_invalid_skeleton_falls_back_to_pending_and_emits_reason() {
        let mut segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            0,
        );
        segment.remaining_skeleton = Some(vec![skeleton_step(vec![
            crate::htn::BeliefPredicate::BountyExpired {
                bounty: crate::htn::EntityTemplate::Fixed(TARGET),
            },
        ])]);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
            Some(&mut traces),
        )
        .expect("invalid skeleton should still resume through pending fallback");

        assert_eq!(resumed.entry.phase, AgendaPhase::Pending);
        assert_eq!(
            resumed
                .entry
                .partial_plan_segment
                .as_ref()
                .and_then(|segment| segment.remaining_skeleton.as_ref()),
            None
        );
        assert_eq!(
            traces[0].decision,
            PartialPlanResumeDecision::FallbackToReplanInvalid(
                SkeletonRevalidationReason::UnsupportedPredicate
            )
        );
        assert_eq!(
            traces[0].per_step_verdicts,
            vec![SkeletonRevalidationVerdict::Invalid(
                SkeletonRevalidationReason::UnsupportedPredicate
            )]
        );
        assert_eq!(traces[0].seeded_ops, None);
    }

    #[test]
    fn try_resume_with_no_skeleton_falls_back_to_pending_and_emits_trace() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            0,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
            Some(&mut traces),
        )
        .expect("no-skeleton segment should resume through pending fallback");

        assert_eq!(resumed.entry.phase, AgendaPhase::Pending);
        assert_eq!(
            traces[0].decision,
            PartialPlanResumeDecision::FallbackToReplanNoSkeleton
        );
        assert!(traces[0].per_step_verdicts.is_empty());
        assert_eq!(traces[0].seeded_ops, None);
    }

    #[test]
    fn try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            1,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(13),
            3,
            Some(&mut traces),
        );

        assert_eq!(resumed, None);
        assert!(state.suspended.contains_key(&key));
        assert!(traces.is_empty());
    }

    #[test]
    fn coordination_barrier_watch_list_indexes_suspended_segments_by_resource() {
        let segment = coordination_barrier_segment(CONTESTED_RESOURCE);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));

        assert_eq!(
            super::coordination_barrier_watch_list(&state),
            vec![super::CoordinationBarrierWatch {
                contested_resource: CONTESTED_RESOURCE,
                entries: vec![key],
            }]
        );
    }

    #[test]
    fn coordination_barrier_resume_waits_while_facility_grant_belongs_to_other_actor() {
        let other_actor = EntityId {
            slot: 55,
            generation: 0,
        };
        let segment = coordination_barrier_segment(CONTESTED_RESOURCE);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let view = ResumeBeliefView {
            contention_policies: BTreeSet::from([CONTESTED_RESOURCE]),
            facility_grants: BTreeMap::from([(CONTESTED_RESOURCE, contention_grant(other_actor))]),
            ..ResumeBeliefView::default()
        };

        let resumed = super::try_resume_partial_plan(&mut state, AGENT, &view, Tick(14), 3);

        assert_eq!(resumed, None);
        assert!(state.suspended.contains_key(&key));
    }

    #[test]
    fn coordination_barrier_resume_fires_when_facility_grant_is_available_again() {
        let segment = coordination_barrier_segment(CONTESTED_RESOURCE);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let view = ResumeBeliefView {
            contention_policies: BTreeSet::from([CONTESTED_RESOURCE]),
            ..ResumeBeliefView::default()
        };

        let resumed = super::try_resume_partial_plan(&mut state, AGENT, &view, Tick(14), 3)
            .expect("cleared facility grant should satisfy coordination resume");

        assert_eq!(resumed.key, key);
        assert_eq!(resumed.entry.phase, AgendaPhase::Pending);
        assert_eq!(resumed.segment.resume_attempt_count, 1);
        assert_eq!(resumed.segment.last_resume_attempt_tick, Some(Tick(14)));
        assert!(state.suspended.is_empty());
    }

    #[test]
    fn coordination_barrier_resume_fires_when_facility_grant_promotes_to_actor() {
        let segment = coordination_barrier_segment(CONTESTED_RESOURCE);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let view = ResumeBeliefView {
            contention_policies: BTreeSet::from([CONTESTED_RESOURCE]),
            facility_grants: BTreeMap::from([(CONTESTED_RESOURCE, contention_grant(AGENT))]),
            ..ResumeBeliefView::default()
        };

        let resumed = super::try_resume_partial_plan(&mut state, AGENT, &view, Tick(14), 3)
            .expect("promoted facility grant should satisfy coordination resume");

        assert_eq!(resumed.key, key);
        assert!(state.suspended.is_empty());
    }

    #[test]
    fn tick_agenda_keeps_unsatisfied_partial_segment_suspended_despite_fresh_candidate() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            1,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment.clone()));
        let fresh = vec![agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            80,
            Tick(13),
        )];

        let profile = default_profile();
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            fresh,
            &MockGoalBeliefView::default(),
            &DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &profile,
                switch_margin: default_switch_margin(),
            },
            Tick(13),
        );

        assert_eq!(transitions.ordered_candidates, Vec::new());
        assert!(state.committed.is_none());
        let suspended = state
            .suspended
            .get(&key)
            .expect("partial segment should remain suspended until its resume condition holds");
        assert_eq!(suspended.partial_plan_segment, Some(segment));
    }

    #[test]
    fn try_resume_partial_plan_abandons_before_resume_when_abandon_condition_holds() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            vec![IntentionAbandonCondition::OpportunityForeverGone(
                OpportunityAnchor::Entity(TARGET),
            )],
            1,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
            Some(&mut traces),
        );

        assert_eq!(resumed, None);
        assert!(state.suspended.is_empty());
        assert_eq!(traces[0].decision, PartialPlanResumeDecision::Abandoned);
    }

    #[test]
    fn try_resume_partial_plan_abandons_when_resume_attempt_exceeds_patience_limit() {
        let segment = partial_plan_segment(
            vec![IntentionResumeCondition::TickElapsed(4)],
            Vec::new(),
            3,
        );
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut traces = Vec::new();

        let resumed = super::try_resume_partial_plan_with_trace(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
            Some(&mut traces),
        );

        assert_eq!(resumed, None);
        assert!(state.suspended.is_empty());
        assert_eq!(traces[0].decision, PartialPlanResumeDecision::Abandoned);
    }

    #[test]
    fn information_barrier_spawns_social_motive_ask_witness_companion() {
        let subject = EntityId {
            slot: 4,
            generation: 0,
        };
        let witness = EntityId {
            slot: 5,
            generation: 0,
        };
        let segment = information_barrier_segment(subject);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let beliefs = information_barrier_beliefs(subject, witness);

        let spawned =
            super::spawn_information_barrier_companions(AGENT, &mut state, &beliefs, Tick(12));

        assert_eq!(spawned.len(), 1);
        let companion = state
            .pending
            .get(&spawned[0])
            .expect("companion should be inserted as pending");
        assert_eq!(
            companion.origin,
            AgendaOrigin::Companion {
                primary: key,
                slot: SlotKind::SocialMotive,
            }
        );
        assert_eq!(
            companion.offer.key.kind,
            GoalKind::AskWitness {
                witness,
                topic: TellTopic::EntityBelief { subject },
            }
        );
        assert_eq!(companion.offer.anchor, OpportunityAnchor::Entity(witness));
        assert_eq!(companion.partial_plan_segment, None);
    }

    #[test]
    fn information_barrier_does_not_spawn_without_plausible_witness() {
        let subject = EntityId {
            slot: 4,
            generation: 0,
        };
        let witness = EntityId {
            slot: 5,
            generation: 0,
        };
        let segment = information_barrier_segment(subject);
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let mut beliefs = information_barrier_beliefs(subject, witness);
        beliefs.known_entity_beliefs.clear();
        beliefs.entity_places.remove(&subject);

        let spawned =
            super::spawn_information_barrier_companions(AGENT, &mut state, &beliefs, Tick(12));

        assert!(spawned.is_empty());
        assert!(state.pending.is_empty());
        assert!(state.suspended.contains_key(&key));
    }

    #[test]
    fn abandoning_information_barrier_primary_cancels_companion() {
        let subject = EntityId {
            slot: 4,
            generation: 0,
        };
        let witness = EntityId {
            slot: 5,
            generation: 0,
        };
        let mut segment = information_barrier_segment(subject);
        segment.abandon_conditions = vec![IntentionAbandonCondition::PatienceExhausted];
        segment.resume_attempt_count = 3;
        let key = OpportunityKey {
            goal_key: segment.goal.key,
            anchor: segment.goal.anchor,
        };
        let mut state = AgendaState::default();
        state
            .suspended
            .insert(key, suspended_partial_entry(segment));
        let companion_goal = GoalKind::AskWitness {
            witness,
            topic: TellTopic::EntityBelief { subject },
        };
        let mut companion = agenda_entry(
            companion_goal,
            OpportunityAnchor::Entity(witness),
            50,
            Tick(11),
        );
        companion.origin = AgendaOrigin::Companion {
            primary: key,
            slot: SlotKind::SocialMotive,
        };
        state.pending.insert(companion.key, companion);

        let resumed = super::try_resume_partial_plan(
            &mut state,
            AGENT,
            &ResumeBeliefView::default(),
            Tick(14),
            3,
        );

        assert_eq!(resumed, None);
        assert!(state.suspended.is_empty());
        assert!(state.pending.is_empty());
    }

    #[test]
    fn merge_fresh_offer_with_same_key_refreshes_without_duplicate() {
        let mut state = AgendaState::default();
        let mut committed = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Entity(TARGET),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        state.committed = Some(committed);
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(2) });
        state.pending.insert(pending.key, pending.clone());

        let fresh = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            40,
            Tick(9),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![fresh],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(state.pending.len(), 1);
        let refreshed = state
            .pending
            .get(&pending.key)
            .expect("pending entry missing");
        assert_eq!(refreshed.last_reconsidered_tick, Tick(10));
        assert_eq!(refreshed.motive_score, 40);
        assert_eq!(transitions.demoted_to_pending, vec![pending.key]);
    }

    #[test]
    fn committed_goal_suppresses_parallel_same_goal_pending_anchor() {
        let mut state = AgendaState::default();
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        state.committed = Some(committed);

        let fresh = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Entity(TARGET),
            40,
            Tick(9),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![fresh],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(state.pending.is_empty());
        assert!(transitions.demoted_to_pending.is_empty());
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key.anchor),
            Some(OpportunityAnchor::Place(PLACE))
        );
    }

    #[test]
    fn revival_trigger_commodity_available_fires_when_belief_confirms_quantity() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::CommodityAvailable {
            place: PLACE,
            kind: CommodityKind::Bread,
            min: Quantity(3),
        });
        state.pending.insert(pending.key, pending.clone());

        let mut beliefs = MockGoalBeliefView::default();
        beliefs
            .controlled_quantities
            .insert((AGENT, PLACE, CommodityKind::Bread), Quantity(5));

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &beliefs,
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.revived, vec![pending.key]);
        assert!(state.pending.is_empty());
        assert_eq!(
            transitions.commit_transition,
            CommitTransition::Committed {
                new_key: pending.key,
                previous_key: None,
            }
        );
    }

    #[test]
    fn kill_condition_tick_expiry_drops_entry_on_or_after_expiry() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::TickExpiry { at_tick: Tick(5) };
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(5),
        );

        assert!(state.pending.is_empty());
        assert_eq!(transitions.killed, vec![pending]);
    }

    #[test]
    fn capacity_overflow_evicts_smallest_last_reconsidered_tick() {
        let mut state = AgendaState::default();
        let mut committed = agenda_entry(
            GoalKind::FreeCarryCapacity,
            OpportunityAnchor::None,
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        state.committed = Some(committed);
        let mut older = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        older.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(2) });
        let mut newer = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            20,
            Tick(3),
        );
        newer.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(4) });
        state.pending.insert(older.key, older.clone());
        state.pending.insert(newer.key, newer.clone());
        let mut remembered = agenda_entry(GoalKind::Wash, OpportunityAnchor::None, 30, Tick(2));
        remembered.phase = AgendaPhase::Suspended;
        remembered.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(5) });
        state.suspended.insert(remembered.key, remembered.clone());

        let mut profile = default_profile();
        profile.pending_capacity = 2;

        let fresh = agenda_entry(GoalKind::Wash, OpportunityAnchor::None, 40, Tick(10));
        let _ = tick_agenda(
            AGENT,
            &mut state,
            vec![fresh.clone()],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &profile,
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(state.pending.len(), 2);
        assert!(!state.pending.contains_key(&older.key));
        assert!(state.pending.contains_key(&newer.key));
        let refreshed = state
            .pending
            .get(&remembered.key)
            .expect("remembered agenda entry should be retained");
        assert_eq!(refreshed.motive_score, fresh.motive_score);
    }

    #[test]
    fn revival_cooldown_blocks_re_promotion_within_window() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(7),
        );
        pending.last_reconsidered_tick = Tick(7);
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(0) });
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.revived.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(0) });
        state.pending.insert(pending.key, pending.clone());

        let blocker_key = worldwake_core::BlockerKey {
            goal_key: pending.offer.key,
            place: Some(PLACE),
            target: None,
            action_def: None,
        };
        let mut discrepancy_memory = worldwake_core::DiscrepancyMemory::default();
        discrepancy_memory.record(worldwake_core::DiscrepancyEntry {
            scope: blocker_key.into(),
            discrepancy: worldwake_core::Discrepancy::BeliefStale,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            source_event: None,
            clearing_condition: worldwake_core::DiscrepancyClearing::TtlExpiry,
        });

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &discrepancy_memory,
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.revived.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn obligation_resolved_kill_condition_clears_missing_expectation() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::ObligationResolved {
            expectation: ExpectationId(9),
        };
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.killed, vec![pending]);
    }

    #[test]
    fn obligation_resolved_kill_condition_preserves_active_expectation() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::ObligationResolved {
            expectation: ExpectationId(9),
        };
        state.pending.insert(pending.key, pending.clone());

        let mut store = ExpectationStore::default();
        store.records.insert(
            ExpectationId(9),
            ExpectationRecord {
                id: ExpectationId(9),
                owner: AGENT,
                subject: TARGET,
                expected_place: PLACE,
                deadline_tick: Tick(20),
                grace_ticks: 0,
                basis: ExpectationBasis::RoutineReturn,
                state: ExpectationState::Active,
                created_tick: Tick(1),
            },
        );
        let beliefs = MockGoalBeliefView {
            expectation_store: Some(store),
            ..MockGoalBeliefView::default()
        };

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &beliefs,
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.killed.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn unchanged_commit_does_not_recommit_when_current_goal_survives() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            50,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            54,
            Tick(10),
        );
        let transitions: AgendaTransitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.commit_transition, CommitTransition::Unchanged);
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(committed_key)
        );
    }

    #[test]
    fn same_class_margin_blocks_commit_switch_when_delta_is_too_small() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            109,
            Tick(10),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.commit_transition, CommitTransition::Unchanged);
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(committed_key)
        );
    }

    #[test]
    fn same_class_margin_switches_commit_when_delta_clears_threshold() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            110,
            Tick(10),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger.clone()],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(
            transitions.commit_transition,
            CommitTransition::Committed {
                new_key: challenger.key,
                previous_key: Some(committed_key),
            }
        );
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(challenger.key)
        );
        assert!(
            state.pending.contains_key(&committed_key),
            "previously committed goal should return to pending after a margin-clearing switch"
        );
    }

    #[test]
    fn classify_rejection_missing_observation_for_commodity_goal_uses_commodity_trigger() {
        assert_eq!(
            classify(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: worldwake_core::CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                OpportunityAnchor::Place(PLACE),
                Discrepancy::MissingObservation,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CommodityAvailable {
                    place: PLACE,
                    kind: CommodityKind::Bread,
                    min: Quantity(1),
                },
            }
        );
    }

    #[test]
    fn classify_rejection_missing_observation_for_target_goal_uses_target_present_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(TARGET, PLACE);
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::MissingObservation,
                },
                &goal_offer(
                    GoalKind::SearchForMissing {
                        subject: TARGET,
                        last_seen: None,
                    },
                    OpportunityAnchor::Entity(TARGET),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TargetPresent {
                    target: TARGET,
                    place: PLACE,
                },
            }
        );
    }

    #[test]
    fn classify_rejection_route_unknown_uses_route_learned_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(AGENT, PLACE);
        beliefs.entity_places.insert(
            TARGET,
            EntityId {
                slot: 4,
                generation: 0,
            },
        );
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::RouteUnknown,
                },
                &goal_offer(
                    GoalKind::SearchForMissing {
                        subject: TARGET,
                        last_seen: None
                    },
                    OpportunityAnchor::Entity(TARGET)
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::RouteLearned {
                    from: PLACE,
                    to: EntityId {
                        slot: 4,
                        generation: 0
                    },
                },
            }
        );
    }

    #[test]
    fn classify_rejection_partial_execution_drift_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::PartialExecutionDrift,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_belief_stale_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::BeliefStale,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_no_willing_counterparty_uses_counterparty_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(TARGET, PLACE);
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::NoWillingCounterparty,
                },
                &goal_offer(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: worldwake_core::CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    OpportunityAnchor::Entity(TARGET),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CounterpartyAvailable {
                    counterparty: TARGET,
                    place: PLACE,
                },
            }
        );
    }

    #[test]
    fn classify_rejection_search_budget_exhausted_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::SearchBudgetExhausted,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_method_failure_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::MethodFailure(worldwake_core::MethodFailureContext {
                    method_id: worldwake_core::MethodSchemaId(7),
                    kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                    subgoal_index: Some(2),
                }),
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_belief_contradicted_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::BeliefContradicted,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_no_legal_binding_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::NoLegalBinding,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_improper_planning_state_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::ImproperPlanningState,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_satisfied_move_cargo_short_circuits_reason_mapping() {
        let destination = EntityId {
            slot: 9,
            generation: 0,
        };
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(destination, destination);
        beliefs
            .controlled_quantities
            .insert((AGENT, destination, CommodityKind::Bread), Quantity(2));
        beliefs
            .demand_memory
            .insert(AGENT, vec![demand(destination, CommodityKind::Bread, 2)]);

        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::NoLegalBinding,
                },
                &goal_offer(
                    GoalKind::MoveCargo {
                        commodity: CommodityKind::Bread,
                        destination,
                    },
                    OpportunityAnchor::Place(destination),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::Satisfied
        );
    }
}
