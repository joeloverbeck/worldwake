use crate::{
    decision_trace::{
        BanditCandidateOmission, BanditCandidateOmissionReason, BanditGoalFamily,
        CandidateEvidenceContributor, CandidateEvidenceExclusion, CandidateEvidenceExclusionReason,
        CandidateEvidenceKind, CandidateEvidenceTrace, CandidateLegalityTrace, DesireFullyBlocked,
        PoliticalCandidateOmission, PoliticalCandidateOmissionReason, PoliticalGoalFamily,
        SocialCandidateOmission,
    },
    derive_danger_pressure,
    enterprise::{analyze_candidate_enterprise, restock_gap_at_destination, EnterpriseSignals},
    institutional_queries::consulted_office_holder_read_for_record_data,
    knowledge_path::{
        BeliefAspect, BeliefProvenance, InstitutionalBeliefProvenance, KnowledgePath,
        SelfKnowledgeProvenance,
    },
    pressure::is_bandit_raid_deterred_by_wounds,
    theft::assess_theft_deterrence,
    GroundedGoal,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use worldwake_core::{
    current_institutional_belief_topics, load_per_unit,
    social_observation_is_redundant_for_listener, tell_subject_is_directly_observable_by_listener,
    BelievedEntityState, BelievedInstitutionalClaim, BlockedIntentMemory, CommodityKind,
    CommodityPurpose, DriveThresholds, EligibilityRule, EntityId, EntityKind, GoalKey, GoalKind,
    HomeostaticNeedId, HomeostaticNeeds, InstitutionalBeliefKey, InstitutionalBeliefRead,
    InstitutionalClaim, InstitutionalKnowledgeSource, OfficeData, OpportunityAnchor,
    OpportunityKey, PerceptionSource, PunishmentFineSelectionTrace, PunishmentFineTraceFacts,
    PunishmentKind, Quantity, RecordData, RecordKind, SocialObservation, SocialObservationDetail,
    TellTopic, TheftFacts, Tick, ViolationId, ViolationKind, ViolationMemory,
};
use worldwake_sim::{
    listener_aware_tell_topic_selection, GoalBeliefView, RecipeDefinition, RecipeRegistry,
    TellTopicOmissionReason,
};

#[derive(Clone, Default)]
struct Evidence {
    entities: BTreeSet<EntityId>,
    places: BTreeSet<EntityId>,
}

impl Evidence {
    fn with_entity(entity: EntityId) -> Self {
        Self {
            entities: BTreeSet::from([entity]),
            places: BTreeSet::new(),
        }
    }

    fn with_place(place: EntityId) -> Self {
        Self {
            entities: BTreeSet::new(),
            places: BTreeSet::from([place]),
        }
    }

    fn merge(&mut self, other: Self) {
        self.entities.extend(other.entities);
        self.places.extend(other.places);
    }

    fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.places.is_empty()
    }
}

#[derive(Default)]
struct EvidenceTrace {
    contributors: BTreeSet<CandidateEvidenceContributor>,
    exclusions: BTreeSet<CandidateEvidenceExclusion>,
    knowledge_path: KnowledgePath,
    legality: Option<CandidateLegalityTrace>,
}

impl EvidenceTrace {
    fn contributor(&mut self, kind: CandidateEvidenceKind, place: EntityId, entity: EntityId) {
        self.contributors.insert(CandidateEvidenceContributor {
            kind,
            place,
            entity,
        });
    }

    fn exclusion(
        &mut self,
        kind: CandidateEvidenceKind,
        place: EntityId,
        entity: EntityId,
        reason: CandidateEvidenceExclusionReason,
    ) {
        self.exclusions.insert(CandidateEvidenceExclusion {
            kind,
            place,
            entity,
            reason,
        });
    }

    fn merge(&mut self, other: Self) {
        self.contributors.extend(other.contributors);
        self.exclusions.extend(other.exclusions);
        self.knowledge_path
            .entity_beliefs
            .extend(other.knowledge_path.entity_beliefs);
        self.knowledge_path
            .self_knowledge
            .extend(other.knowledge_path.self_knowledge);
        self.knowledge_path
            .institutional_beliefs
            .extend(other.knowledge_path.institutional_beliefs);
    }

    fn is_empty(&self) -> bool {
        self.contributors.is_empty() && self.exclusions.is_empty()
    }

    fn into_public(self, opportunity: OpportunityKey) -> CandidateEvidenceTrace {
        CandidateEvidenceTrace {
            opportunity,
            contributors: self.contributors.into_iter().collect(),
            exclusions: self.exclusions.into_iter().collect(),
            knowledge_path: self.knowledge_path,
            legality: self.legality,
        }
    }
}

#[derive(Copy, Clone)]
struct AcquisitionSearchOptions<'a> {
    include_recipes: bool,
    visited_commodities: &'a BTreeSet<CommodityKind>,
}

struct GenerationContext<'a> {
    view: &'a dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    travel_horizon: u8,
    enterprise: EnterpriseSignals,
    blocked: &'a BlockedIntentMemory,
    violation_memory: &'a ViolationMemory,
    recipes: &'a RecipeRegistry,
    current_tick: Tick,
    tracing_enabled: bool,
}

#[derive(Default)]
pub(crate) struct CandidateGenerationDiagnostics {
    pub omitted_political: Vec<PoliticalCandidateOmission>,
    pub omitted_bandit: Vec<BanditCandidateOmission>,
    pub omitted_social: Vec<SocialCandidateOmission>,
    pub evidence: BTreeMap<OpportunityKey, CandidateEvidenceTrace>,
    pub fully_blocked_desires: Vec<DesireFullyBlocked>,
}

pub(crate) struct CandidateGenerationResult {
    pub candidates: Vec<GroundedGoal>,
    pub diagnostics: CandidateGenerationDiagnostics,
    /// Violations detected during candidate generation that should be recorded
    /// in the agent's [`ViolationMemory`] by the caller. Generation itself is
    /// side-effect-free; the caller applies these after the read phase.
    pub pending_violations: Vec<PendingViolationRecord>,
}

/// A violation detected during candidate generation, to be recorded in
/// [`ViolationMemory`] by the caller after the generation pass completes.
pub(crate) struct PendingViolationRecord {
    pub id: ViolationId,
    pub kind: ViolationKind,
    pub observed_tick: Tick,
    pub ttl: u32,
}

#[must_use]
pub fn generate_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal> {
    let empty_vm = ViolationMemory::default();
    generate_candidates_with_travel_horizon(
        view,
        agent,
        blocked,
        &empty_vm,
        recipes,
        current_tick,
        6,
        false,
    )
    .candidates
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_candidates_with_travel_horizon(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    violation_memory: &ViolationMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,
    tracing_enabled: bool,
) -> CandidateGenerationResult {
    if view.is_dead(agent) || !view.is_alive(agent) {
        return CandidateGenerationResult {
            candidates: Vec::new(),
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
        };
    }

    let mut candidates = Vec::new();
    let mut diagnostics = CandidateGenerationDiagnostics::default();
    let needs = view.homeostatic_needs(agent);
    let thresholds = view.drive_thresholds(agent);
    let place = view.effective_place(agent);
    let ctx = GenerationContext {
        view,
        agent,
        place,
        travel_horizon,
        enterprise: analyze_candidate_enterprise(view, agent, place),
        blocked,
        violation_memory,
        recipes,
        current_tick,
        tracing_enabled,
    };

    emit_need_candidates(&mut candidates, &mut diagnostics, &ctx, needs, thresholds);
    emit_production_candidates(&mut candidates, &mut diagnostics, &ctx, needs, thresholds);
    emit_enterprise_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_combat_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_crime_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_social_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_patrol_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_political_candidates(&mut candidates, &mut diagnostics, &ctx);
    emit_recorded_violation_candidates(&mut candidates, &mut diagnostics, &ctx);
    let pending_violations =
        emit_expectation_violation_candidates(&mut candidates, &mut diagnostics, &ctx);

    let candidates = filter_blocked_candidates(candidates, blocked, current_tick, &mut diagnostics);

    CandidateGenerationResult {
        candidates,
        diagnostics,
        pending_violations,
    }
}

fn filter_blocked_candidates(
    candidates: Vec<GroundedGoal>,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    diagnostics: &mut CandidateGenerationDiagnostics,
) -> Vec<GroundedGoal> {
    let mut blocked_by_goal: BTreeMap<GoalKey, Vec<OpportunityKey>> = BTreeMap::new();
    let mut emitted_counts: BTreeMap<GoalKey, usize> = BTreeMap::new();
    let mut surviving = Vec::new();

    for candidate in candidates {
        *emitted_counts.entry(candidate.key).or_default() += 1;
        if is_candidate_blocked(&candidate, blocked, current_tick) {
            blocked_by_goal
                .entry(candidate.key)
                .or_default()
                .push(OpportunityKey {
                    goal_key: candidate.key,
                    anchor: candidate.anchor,
                });
            continue;
        }
        surviving.push(candidate);
    }

    diagnostics.fully_blocked_desires = blocked_by_goal
        .into_iter()
        .filter_map(|(goal_key, mut blocked_opportunities)| {
            let emitted = emitted_counts.get(&goal_key).copied().unwrap_or_default();
            if emitted == 0 || blocked_opportunities.len() != emitted {
                return None;
            }
            blocked_opportunities.sort();
            Some(DesireFullyBlocked {
                goal_key,
                blocked_opportunities,
            })
        })
        .collect();

    surviving
}

fn is_candidate_blocked(
    candidate: &GroundedGoal,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> bool {
    blocked.intents.values().any(|intent| {
        intent.blocker_key.goal_key == candidate.key
            && intent.expires_tick > current_tick
            && intent.blocks_goal_generation()
            && candidate_matches_blocker(candidate, &intent.blocker_key)
    })
}

fn candidate_matches_blocker(
    candidate: &GroundedGoal,
    blocker: &worldwake_core::BlockerKey,
) -> bool {
    if blocker.place.is_none() && blocker.target.is_none() && blocker.action_def.is_none() {
        return true;
    }

    if let Some(place) = blocker.place {
        let anchor_matches =
            matches!(candidate.anchor, OpportunityAnchor::Place(anchor) if anchor == place);
        if !anchor_matches && !candidate.evidence_places.contains(&place) {
            return false;
        }
    }

    if let Some(target) = blocker.target {
        let anchor_matches =
            matches!(candidate.anchor, OpportunityAnchor::Entity(anchor) if anchor == target);
        if !anchor_matches && !candidate.evidence_entities.contains(&target) {
            return false;
        }
    }

    true
}

fn emit_need_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
) {
    let (Some(needs), Some(thresholds)) = (needs, thresholds) else {
        return;
    };

    emit_self_consume_candidates(candidates, diagnostics, ctx, needs, thresholds);
    emit_sleep_goal(candidates, diagnostics, ctx, needs, thresholds);
    emit_relieve_goal(candidates, diagnostics, ctx, needs, thresholds);
    emit_wash_goal(candidates, diagnostics, ctx, needs, thresholds);
}

fn emit_production_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
) {
    emit_produce_goals(candidates, diagnostics, ctx, needs, thresholds);
}

fn emit_enterprise_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    emit_restock_goals(candidates, diagnostics, ctx);
    emit_move_cargo_goals(candidates, diagnostics, ctx);
}

fn emit_combat_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    emit_raid_target_goals(candidates, diagnostics, ctx);
    emit_engage_hostile_goals(candidates, diagnostics, ctx);
    emit_reduce_danger_goal(candidates, diagnostics, ctx);
    emit_care_goals(candidates, diagnostics, ctx);
    emit_loot_goals(candidates, diagnostics, ctx);
    emit_bury_goals(candidates, diagnostics, ctx);
}

fn emit_crime_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    emit_theft_candidates(candidates, diagnostics, ctx);
    emit_justice_candidates(candidates, diagnostics, ctx);
}

fn emit_patrol_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let (Some(route), Some(_profile)) = (
        ctx.view.patrol_route(ctx.agent),
        ctx.view.patrol_profile(ctx.agent),
    ) else {
        return;
    };
    let Some(&place) = route.assigned_places.get(route.current_index) else {
        return;
    };

    emit_candidate_with_trace(
        candidates,
        diagnostics,
        GoalKind::Patrol { place },
        OpportunityAnchor::Place(place),
        Evidence::with_place(place),
        EvidenceTrace::default(),
    );
}

fn emit_justice_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(profile) = ctx.view.justice_disposition_profile(ctx.agent) else {
        return;
    };

    let known_social_observations = ctx.view.known_social_observations(ctx.agent);
    let current_crime_case_claims =
        current_institutional_belief_topics(ctx.view.known_institutional_beliefs(ctx.agent));

    emit_accusation_candidates(candidates, diagnostics, ctx, &known_social_observations);
    emit_punishment_candidates(
        candidates,
        diagnostics,
        ctx,
        &current_crime_case_claims,
        profile.fine_severity.value(),
    );
}

fn emit_accusation_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    known_social_observations: &[SocialObservation],
) {
    let known_crime_registers = known_authority_crime_registers(ctx);

    for record in ctx.violation_memory.unresolved_records(ctx.current_tick) {
        let ViolationKind::SuspectedTheft { theft, suspect } = record.kind else {
            continue;
        };

        let mut accused_candidates = BTreeSet::new();
        if let Some(accused) = suspect {
            accused_candidates.insert(accused);
        }
        for observation in known_social_observations {
            if let SocialObservationDetail::SuspectedTheft {
                theft: observed_theft,
                suspect: Some(accused),
            } = observation.detail
            {
                if observed_theft == theft {
                    accused_candidates.insert(accused);
                }
            }
        }

        for accused in accused_candidates {
            if !ctx.view.is_alive(accused) {
                continue;
            }
            for (crime_register, record_data) in &known_crime_registers {
                if record_data.has_accusation_case_for(accused, &theft) {
                    continue;
                }
                let mut evidence = Evidence {
                    entities: BTreeSet::from([accused, theft.missing_entity, *crime_register]),
                    places: BTreeSet::from([theft.expected_place, record_data.home_place]),
                };
                evidence.entities.insert(record_data.issuer);
                emit_candidate_with_trace(
                    candidates,
                    diagnostics,
                    GoalKind::Accuse {
                        crime_register: *crime_register,
                        accused,
                        violation_id: record.id,
                    },
                    OpportunityAnchor::Entity(accused),
                    evidence,
                    EvidenceTrace::default(),
                );
            }
        }
    }
}

fn known_authority_crime_registers(ctx: &GenerationContext<'_>) -> Vec<(EntityId, RecordData)> {
    ctx.view
        .known_entity_beliefs(ctx.agent)
        .into_iter()
        .filter_map(|(entity, _)| {
            (ctx.view.entity_kind(entity) == Some(EntityKind::Record))
                .then_some((entity, ctx.view.record_data(entity)?))
        })
        .filter(|(_, record_data)| {
            record_data.record_kind == RecordKind::CrimeRegister
                && matches!(
                    ctx.view.believed_office_holder(record_data.issuer),
                    InstitutionalBeliefRead::Certain(Some(holder)) if holder == ctx.agent
                )
        })
        .collect()
}

fn emit_punishment_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    current_crime_case_claims: &[BelievedInstitutionalClaim],
    fine_severity_permille: u16,
) {
    for belief in current_crime_case_claims {
        let (
            InstitutionalClaim::Accusation {
                accused,
                violation_id: _violation_id,
                theft,
                ..
            },
            InstitutionalKnowledgeSource::RecordConsultation { record, entry_id },
        ) = (belief.claim, belief.source)
        else {
            continue;
        };

        if !ctx.view.is_alive(accused) {
            continue;
        }

        let Some(record_data) = ctx.view.record_data(record) else {
            continue;
        };
        if record_data.record_kind != RecordKind::CrimeRegister {
            continue;
        }
        let office = record_data.issuer;
        let Some(office_data) = ctx.view.office_data(office) else {
            continue;
        };
        if !matches!(
            ctx.view.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(Some(holder)) if holder == ctx.agent
        ) {
            continue;
        }

        let Some((punishment, legality_trace)) = candidate_punishment_for_case(
            ctx.view,
            ctx.agent,
            &PunishmentCaseContext {
                accused,
                office,
                office_data: &office_data,
                accusation_entry: entry_id,
                theft,
            },
            fine_severity_permille,
        ) else {
            continue;
        };

        let mut evidence = Evidence::with_entity(accused);
        evidence.entities.insert(office);
        evidence.entities.insert(record);
        evidence.places.insert(office_data.jurisdiction);
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            trace
                .knowledge_path
                .institutional_beliefs
                .push(InstitutionalBeliefProvenance {
                    claim: belief.claim,
                    source: belief.source,
                    learned_tick: belief.learned_tick,
                    learned_at: belief.learned_at,
                });
            if let Some(legality_trace) = legality_trace {
                trace.legality = Some(CandidateLegalityTrace::PunishmentFineSelection(
                    legality_trace,
                ));
            }
        }

        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::PunishAccused {
                office,
                accused,
                accusation_entry: entry_id,
                punishment,
            },
            OpportunityAnchor::Entity(accused),
            evidence,
            trace,
        );
    }
}

struct PunishmentCaseContext<'a> {
    accused: EntityId,
    office: EntityId,
    office_data: &'a OfficeData,
    accusation_entry: worldwake_core::RecordEntryId,
    theft: TheftFacts,
}

fn candidate_punishment_for_case(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    case: &PunishmentCaseContext<'_>,
    fine_severity_permille: u16,
) -> Option<(PunishmentKind, Option<PunishmentFineSelectionTrace>)> {
    let fine_amount = Quantity(
        (u64::from(case.theft.quantity.0) * u64::from(fine_severity_permille) / 1000) as u32,
    );
    let actor_place = view.effective_place(agent);
    let accused_place = view.effective_place(case.accused);
    let locally_observed_quantity =
        view.locally_observed_commodity_quantity(agent, case.accused, case.theft.commodity);
    if fine_amount > Quantity(0)
        && actor_place.is_some()
        && actor_place == accused_place
        && locally_observed_quantity >= fine_amount
    {
        return Some((
            PunishmentKind::Fine {
                commodity: case.theft.commodity,
                amount: fine_amount,
            },
            Some(PunishmentFineSelectionTrace {
                facts: PunishmentFineTraceFacts {
                    office: case.office,
                    accusation_entry: case.accusation_entry,
                    accused: case.accused,
                    theft: case.theft,
                    actor_place,
                    accused_place,
                    required_amount: fine_amount,
                },
                locally_observed_quantity,
            }),
        ));
    }

    office_governed_faction_for_accused(view, case.office_data, case.accused)
        .map(|from_faction| (PunishmentKind::Exile { from_faction }, None))
}

fn office_governed_faction_for_accused(
    view: &dyn GoalBeliefView,
    office_data: &OfficeData,
    accused: EntityId,
) -> Option<EntityId> {
    office_data
        .eligibility_rules
        .iter()
        .filter_map(|rule| match rule {
            EligibilityRule::FactionMember(faction)
                if matches!(
                    view.believed_membership(*faction, accused),
                    InstitutionalBeliefRead::Certain(true)
                ) =>
            {
                Some(*faction)
            }
            EligibilityRule::FactionMember(_) => None,
        })
        .min()
}

fn emit_social_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    emit_regroup_with_faction_goals(candidates, diagnostics, ctx);

    let Some(place) = ctx.place else {
        return;
    };
    let Some(profile) = ctx.view.tell_profile(ctx.agent) else {
        return;
    };
    let known_beliefs = ctx.view.known_entity_beliefs(ctx.agent);
    let known_social_observations = ctx.view.known_social_observations(ctx.agent);
    let known_institutional_beliefs =
        current_institutional_belief_topics(ctx.view.known_institutional_beliefs(ctx.agent));

    for listener in social_listeners_at(ctx.view, ctx.agent, place) {
        let relayable_entity_beliefs = known_beliefs
            .iter()
            .filter(|(subject, _)| {
                let claim_first_subject = matches!(
                    ctx.view.entity_kind(*subject),
                    Some(EntityKind::Office | EntityKind::Record)
                ) && known_institutional_beliefs.iter().any(|belief| {
                    worldwake_core::institutional_claim_subject_entity(belief.claim) == *subject
                });
                if claim_first_subject {
                    return false;
                }
                let directly_observable =
                    subject_is_listener_observable_entity_belief(ctx.view, listener, *subject);
                if directly_observable {
                    diagnostics.omitted_social.push(SocialCandidateOmission {
                        listener,
                        topic: TellTopic::EntityBelief { subject: *subject },
                        reason: TellTopicOmissionReason::DirectlyObservableByListener,
                    });
                }
                !directly_observable
            })
            .cloned()
            .collect::<Vec<_>>();
        let relayable_social_observations = known_social_observations
            .iter()
            .copied()
            .filter(|observation| {
                let redundant = social_observation_is_redundant_for_listener(observation, listener);
                if redundant {
                    diagnostics.omitted_social.push(SocialCandidateOmission {
                        listener,
                        topic: TellTopic::SocialObservation {
                            observation: *observation,
                        },
                        reason: TellTopicOmissionReason::ListenerParticipatedInObservation,
                    });
                }
                !redundant
            })
            .collect::<Vec<_>>();
        let selection = listener_aware_tell_topic_selection(
            relayable_entity_beliefs.clone(),
            relayable_social_observations,
            known_institutional_beliefs.clone(),
            profile.max_relay_chain_len,
            profile.max_tell_candidates,
            |topic| {
                ctx.view
                    .recipient_knowledge_status(ctx.agent, listener, topic)
                    .unwrap_or(worldwake_core::RecipientKnowledgeStatus::UnknownToSpeaker)
            },
        );
        diagnostics
            .omitted_social
            .extend(
                selection
                    .omitted
                    .into_iter()
                    .map(|omission| SocialCandidateOmission {
                        listener,
                        topic: omission.topic,
                        reason: omission.reason,
                    }),
            );

        for topic in selection.selected.iter().copied() {
            let mut evidence = Evidence::with_entity(listener);
            evidence.places.insert(place);
            let mut trace = EvidenceTrace::default();
            trace.contributor(CandidateEvidenceKind::Listener, place, listener);
            if let TellTopic::EntityBelief { subject } = topic {
                evidence.entities.insert(subject);
                trace.contributor(CandidateEvidenceKind::TellSubject, place, subject);
                if ctx.tracing_enabled {
                    if let Some((_, state)) = known_beliefs.iter().find(|(id, _)| *id == subject) {
                        trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                            subject,
                            aspect: BeliefAspect::LocationAt { place },
                            source: state.source,
                            observed_tick: state.observed_tick,
                        });
                    }
                }
            } else if let TellTopic::InstitutionalClaim { claim } = topic {
                if ctx.tracing_enabled {
                    if let Some(belief) = known_institutional_beliefs
                        .iter()
                        .filter(|belief| belief.claim == claim)
                        .max_by_key(|belief| {
                            (
                                std::cmp::Reverse(
                                    worldwake_core::institutional_knowledge_chain_len(
                                        belief.source,
                                    ),
                                ),
                                belief.learned_tick,
                                belief.learned_at,
                            )
                        })
                    {
                        trace.knowledge_path.institutional_beliefs.push(
                            InstitutionalBeliefProvenance {
                                claim,
                                source: belief.source,
                                learned_tick: belief.learned_tick,
                                learned_at: belief.learned_at,
                            },
                        );
                    }
                }
            }
            emit_candidate_with_trace(
                candidates,
                diagnostics,
                GoalKind::ShareBelief { listener, topic },
                OpportunityAnchor::Entity(listener),
                evidence,
                trace,
            );
        }
    }
}

fn emit_regroup_with_faction_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(current_place) = ctx.place else {
        return;
    };

    for faction in ctx.view.bandit_factions_of(ctx.agent) {
        if ctx
            .view
            .locally_observed_bandit_camp_faction_at(ctx.agent, current_place)
            == Some(faction)
            && !ctx.view.has_wounds(ctx.agent)
            && ctx.view.visible_hostiles_for(ctx.agent).is_empty()
        {
            diagnostics.omitted_bandit.push(BanditCandidateOmission {
                family: BanditGoalFamily::RegroupWithFaction,
                faction,
                reason: BanditCandidateOmissionReason::AlreadySafeInObservedActiveCamp,
            });
            continue;
        }

        let InstitutionalBeliefRead::Certain(Some(rally_place)) =
            ctx.view.believed_faction_rally_point(faction)
        else {
            diagnostics.omitted_bandit.push(BanditCandidateOmission {
                family: BanditGoalFamily::RegroupWithFaction,
                faction,
                reason: BanditCandidateOmissionReason::MissingRallyBelief,
            });
            continue;
        };
        if rally_place == current_place {
            if ctx
                .view
                .locally_observed_bandit_camp_faction_at(ctx.agent, current_place)
                == Some(faction)
            {
                diagnostics.omitted_bandit.push(BanditCandidateOmission {
                    family: BanditGoalFamily::EstablishBanditCamp,
                    faction,
                    reason: BanditCandidateOmissionReason::AlreadyAtRallyWithObservedActiveCamp,
                });
                continue;
            }
            if !has_local_controlled_edible_supplies(ctx.view, ctx.agent, current_place) {
                diagnostics.omitted_bandit.push(BanditCandidateOmission {
                    family: BanditGoalFamily::EstablishBanditCamp,
                    faction,
                    reason: BanditCandidateOmissionReason::MissingLocalControlledEdibleSupplies,
                });
                continue;
            }

            let mut evidence = Evidence::with_place(rally_place);
            evidence.entities.insert(faction);
            emit_candidate_with_trace(
                candidates,
                diagnostics,
                GoalKind::EstablishBanditCamp { faction },
                OpportunityAnchor::Place(rally_place),
                evidence,
                EvidenceTrace::default(),
            );
            continue;
        }

        let mut evidence = Evidence::with_place(rally_place);
        evidence.entities.insert(faction);
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            let key = InstitutionalBeliefKey::FactionRallyPointOf { faction };
            if let Some(belief) = ctx
                .view
                .institutional_belief_claims(ctx.agent, key)
                .into_iter()
                .filter(|belief| {
                    matches!(
                        belief.claim,
                        InstitutionalClaim::FactionRallyPoint {
                            faction: claim_faction,
                            rally_place: Some(claim_rally_place),
                            ..
                        } if claim_faction == faction && claim_rally_place == rally_place
                    )
                })
                .max_by_key(|belief| (belief.learned_tick, belief.learned_at))
            {
                trace
                    .knowledge_path
                    .institutional_beliefs
                    .push(InstitutionalBeliefProvenance {
                        claim: belief.claim,
                        source: belief.source,
                        learned_tick: belief.learned_tick,
                        learned_at: belief.learned_at,
                    });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::RegroupWithFaction { faction },
            OpportunityAnchor::Place(rally_place),
            evidence,
            trace,
        );
    }
}

fn has_local_controlled_edible_supplies(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: EntityId,
) -> bool {
    CommodityKind::ALL.into_iter().any(|commodity| {
        view.local_controlled_lots_for(agent, place, commodity)
            .into_iter()
            .any(|item| {
                view.item_lot_consumable_profile(item)
                    .is_some_and(|profile| profile.hunger_relief_per_unit.value() > 0)
            })
    })
}

fn subject_is_listener_observable_entity_belief(
    view: &dyn GoalBeliefView,
    listener: EntityId,
    subject: EntityId,
) -> bool {
    tell_subject_is_directly_observable_by_listener(
        subject,
        view.entity_kind(subject),
        view.effective_place(subject),
        listener,
        view.effective_place(listener),
        view.observation_fidelity(listener),
    )
}

fn emit_political_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let mut known_offices = ctx
        .view
        .known_entity_beliefs(ctx.agent)
        .into_iter()
        .map(|(entity, _)| entity)
        .filter(|entity| ctx.view.entity_kind(*entity) == Some(EntityKind::Office))
        .collect::<std::collections::BTreeSet<_>>();
    known_offices.extend(
        ctx.view
            .known_institutional_beliefs(ctx.agent)
            .into_iter()
            .map(|belief| worldwake_core::institutional_claim_subject_entity(belief.claim))
            .filter(|entity| ctx.view.entity_kind(*entity) == Some(EntityKind::Office)),
    );
    for office in known_offices {
        let Some(office_data) = ctx.view.office_data(office) else {
            continue;
        };
        match office_data.succession_law {
            worldwake_core::SuccessionLaw::Support => {
                let office_evidence = match political_office_evidence(ctx, office, &office_data) {
                    Ok(evidence) => evidence,
                    Err(reason) => {
                        record_office_wide_political_omission(diagnostics, office, reason);
                        continue;
                    }
                };

                emit_claim_office_candidate(
                    candidates,
                    diagnostics,
                    ctx,
                    office,
                    &office_data,
                    &office_evidence,
                );
                emit_support_candidate_goals(
                    candidates,
                    diagnostics,
                    ctx,
                    office,
                    &office_data,
                    &office_evidence,
                );
            }
            worldwake_core::SuccessionLaw::Force => {
                let office_evidence =
                    match force_political_office_evidence(ctx, office, &office_data) {
                        Ok(evidence) => evidence,
                        Err(reason) => {
                            diagnostics
                                .omitted_political
                                .push(PoliticalCandidateOmission {
                                    family: PoliticalGoalFamily::ClaimOffice,
                                    office,
                                    candidate: None,
                                    reason,
                                });
                            diagnostics
                                .omitted_political
                                .push(PoliticalCandidateOmission {
                                    family: PoliticalGoalFamily::SupportCandidateForOffice,
                                    office,
                                    candidate: None,
                                    reason: PoliticalCandidateOmissionReason::ForceSuccessionLaw,
                                });
                            continue;
                        }
                    };
                emit_claim_office_candidate(
                    candidates,
                    diagnostics,
                    ctx,
                    office,
                    &office_data,
                    &office_evidence,
                );
                diagnostics
                    .omitted_political
                    .push(PoliticalCandidateOmission {
                        family: PoliticalGoalFamily::SupportCandidateForOffice,
                        office,
                        candidate: None,
                        reason: PoliticalCandidateOmissionReason::ForceSuccessionLaw,
                    });
            }
        }
    }
}

fn political_office_evidence(
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<Evidence, PoliticalCandidateOmissionReason> {
    if office_data.vacancy_since.is_none() {
        return Err(PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant);
    }

    match ctx.view.believed_office_holder(office) {
        InstitutionalBeliefRead::Certain(None) => Ok(Evidence::default()),
        InstitutionalBeliefRead::Certain(Some(_)) => {
            Err(PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant)
        }
        InstitutionalBeliefRead::Unknown => known_consultable_office_register(ctx, office)
            .ok_or(PoliticalCandidateOmissionReason::OfficeHolderBeliefUnknownNoConsultableRecord),
        InstitutionalBeliefRead::Conflicted(_) => {
            Err(PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted)
        }
    }
}

fn force_political_office_evidence(
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
) -> Result<Evidence, PoliticalCandidateOmissionReason> {
    if office_data.vacancy_since.is_some() {
        return Ok(Evidence::default());
    }
    let hostiles = ctx
        .view
        .hostile_targets_of(ctx.agent)
        .into_iter()
        .collect::<BTreeSet<_>>();
    match ctx.view.believed_force_controller(office) {
        InstitutionalBeliefRead::Certain((None, _)) => Ok(Evidence::default()),
        InstitutionalBeliefRead::Certain((Some(controller), _)) if controller == ctx.agent => {
            Err(PoliticalCandidateOmissionReason::AlreadyDeclaredSupport)
        }
        InstitutionalBeliefRead::Certain((Some(controller), _))
            if hostiles.contains(&controller) =>
        {
            Ok(Evidence::with_entity(controller))
        }
        InstitutionalBeliefRead::Conflicted(_) => {
            Err(PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted)
        }
        _ => Err(PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant),
    }
}

fn known_consultable_office_register(
    ctx: &GenerationContext<'_>,
    office: EntityId,
) -> Option<Evidence> {
    ctx.view
        .known_entity_beliefs(ctx.agent)
        .into_iter()
        .filter_map(|(entity, _)| {
            (ctx.view.entity_kind(entity) == Some(EntityKind::Record))
                .then_some((entity, ctx.view.record_data(entity)?))
        })
        .find_map(|(record, record_data)| {
            (record_data.record_kind == RecordKind::OfficeRegister
                && !matches!(
                    consulted_office_holder_read_for_record_data(&record_data, office),
                    InstitutionalBeliefRead::Unknown
                ))
            .then(|| {
                let mut evidence = Evidence::with_entity(record);
                evidence.places.insert(record_data.home_place);
                evidence
            })
        })
}

fn support_declaration_conflicted(
    view: &dyn GoalBeliefView,
    office: EntityId,
    supporter: EntityId,
) -> bool {
    matches!(
        view.believed_support_declaration(office, supporter),
        InstitutionalBeliefRead::Conflicted(_)
    )
}

fn support_declaration_matches_candidate(
    view: &dyn GoalBeliefView,
    office: EntityId,
    supporter: EntityId,
    candidate: EntityId,
) -> bool {
    matches!(
        view.believed_support_declaration(office, supporter),
        InstitutionalBeliefRead::Certain(Some(current)) if current == candidate
    )
}

fn emit_claim_office_candidate(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
    office_evidence: &Evidence,
) {
    if !candidate_is_eligible(ctx.view, office_data, ctx.agent) {
        diagnostics
            .omitted_political
            .push(PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office,
                candidate: None,
                reason: PoliticalCandidateOmissionReason::ActorNotEligible,
            });
        return;
    }
    if support_declaration_conflicted(ctx.view, office, ctx.agent) {
        diagnostics
            .omitted_political
            .push(PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office,
                candidate: None,
                reason: PoliticalCandidateOmissionReason::SupportDeclarationBeliefConflicted,
            });
        return;
    }
    if support_declaration_matches_candidate(ctx.view, office, ctx.agent, ctx.agent) {
        diagnostics
            .omitted_political
            .push(PoliticalCandidateOmission {
                family: PoliticalGoalFamily::ClaimOffice,
                office,
                candidate: None,
                reason: PoliticalCandidateOmissionReason::AlreadyDeclaredSupport,
            });
        return;
    }

    let mut evidence = office_evidence.clone();
    evidence.entities.insert(office);
    evidence.entities.insert(ctx.agent);
    evidence.places.insert(office_data.jurisdiction);
    let mut trace = EvidenceTrace::default();
    trace.contributor(
        CandidateEvidenceKind::OfficeParticipant,
        office_data.jurisdiction,
        office,
    );
    trace.contributor(
        CandidateEvidenceKind::OfficeParticipant,
        office_data.jurisdiction,
        ctx.agent,
    );
    if ctx.tracing_enabled {
        let claims = ctx.view.institutional_belief_claims(
            ctx.agent,
            InstitutionalBeliefKey::OfficeHolderOf { office },
        );
        for claim in claims {
            trace
                .knowledge_path
                .institutional_beliefs
                .push(InstitutionalBeliefProvenance {
                    claim: claim.claim,
                    source: claim.source,
                    learned_tick: claim.learned_tick,
                    learned_at: claim.learned_at,
                });
        }
    }
    emit_candidate_with_trace(
        candidates,
        diagnostics,
        GoalKind::ClaimOffice { office },
        OpportunityAnchor::Entity(office),
        evidence,
        trace,
    );
}

fn emit_support_candidate_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
    office_evidence: &Evidence,
) {
    let current_declaration_conflicted =
        support_declaration_conflicted(ctx.view, office, ctx.agent);
    for (candidate, _) in ctx.view.known_entity_beliefs(ctx.agent) {
        if candidate == ctx.agent {
            continue;
        }
        let Some(loyalty) = ctx.view.loyalty_to(ctx.agent, candidate) else {
            continue;
        };
        if loyalty == worldwake_core::Permille::new_unchecked(0) {
            continue;
        }
        if !candidate_is_eligible(ctx.view, office_data, candidate) {
            diagnostics
                .omitted_political
                .push(PoliticalCandidateOmission {
                    family: PoliticalGoalFamily::SupportCandidateForOffice,
                    office,
                    candidate: Some(candidate),
                    reason: PoliticalCandidateOmissionReason::CandidateNotEligible,
                });
            continue;
        }
        if current_declaration_conflicted {
            diagnostics
                .omitted_political
                .push(PoliticalCandidateOmission {
                    family: PoliticalGoalFamily::SupportCandidateForOffice,
                    office,
                    candidate: Some(candidate),
                    reason: PoliticalCandidateOmissionReason::SupportDeclarationBeliefConflicted,
                });
            continue;
        }
        if support_declaration_matches_candidate(ctx.view, office, ctx.agent, candidate) {
            diagnostics
                .omitted_political
                .push(PoliticalCandidateOmission {
                    family: PoliticalGoalFamily::SupportCandidateForOffice,
                    office,
                    candidate: Some(candidate),
                    reason: PoliticalCandidateOmissionReason::AlreadyDeclaredSupport,
                });
            continue;
        }

        let mut evidence = office_evidence.clone();
        evidence.entities.insert(office);
        evidence.entities.insert(candidate);
        evidence.places.insert(office_data.jurisdiction);
        let mut trace = EvidenceTrace::default();
        trace.contributor(
            CandidateEvidenceKind::OfficeParticipant,
            office_data.jurisdiction,
            office,
        );
        trace.contributor(
            CandidateEvidenceKind::OfficeParticipant,
            office_data.jurisdiction,
            candidate,
        );
        if ctx.tracing_enabled {
            let claims = ctx.view.institutional_belief_claims(
                ctx.agent,
                InstitutionalBeliefKey::SupportFor {
                    supporter: ctx.agent,
                    office,
                },
            );
            for claim in claims {
                trace
                    .knowledge_path
                    .institutional_beliefs
                    .push(InstitutionalBeliefProvenance {
                        claim: claim.claim,
                        source: claim.source,
                        learned_tick: claim.learned_tick,
                        learned_at: claim.learned_at,
                    });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::SupportCandidateForOffice { office, candidate },
            OpportunityAnchor::Entity(office),
            evidence,
            trace,
        );
    }
}

fn record_office_wide_political_omission(
    diagnostics: &mut CandidateGenerationDiagnostics,
    office: EntityId,
    reason: PoliticalCandidateOmissionReason,
) {
    diagnostics
        .omitted_political
        .push(PoliticalCandidateOmission {
            family: PoliticalGoalFamily::ClaimOffice,
            office,
            candidate: None,
            reason,
        });
    diagnostics
        .omitted_political
        .push(PoliticalCandidateOmission {
            family: PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            candidate: None,
            reason,
        });
}

fn candidate_is_eligible(
    view: &dyn GoalBeliefView,
    office_data: &OfficeData,
    candidate: EntityId,
) -> bool {
    view.entity_kind(candidate) == Some(EntityKind::Agent)
        && view.is_alive(candidate)
        && office_data.eligibility_rules.iter().all(|rule| {
            matches!(
                rule,
                EligibilityRule::FactionMember(faction)
                    if view.believed_membership(*faction, candidate)
                        == InstitutionalBeliefRead::Certain(true)
            )
        })
}

fn emit_engage_hostile_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    if ctx
        .view
        .drive_thresholds(ctx.agent)
        .is_some_and(|thresholds| {
            derive_danger_pressure(ctx.view, ctx.agent) >= thresholds.danger.high()
        })
    {
        return;
    }

    let current_attackers = ctx
        .view
        .current_attackers_of(ctx.agent)
        .into_iter()
        .collect::<BTreeSet<_>>();

    let beliefs = if ctx.tracing_enabled {
        ctx.view.known_entity_beliefs(ctx.agent)
    } else {
        Vec::new()
    };
    let raid_targets = local_raid_targets(ctx.view, ctx.agent, ctx.place)
        .into_iter()
        .collect::<BTreeSet<_>>();

    let local_hostiles = local_hostility_targets(ctx.view, ctx.agent, ctx.place)
        .into_iter()
        .collect::<BTreeSet<_>>();

    for target in &local_hostiles {
        if raid_targets.contains(target) {
            continue;
        }
        if current_attackers.contains(target) {
            continue;
        }

        let mut evidence = Evidence::with_entity(*target);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            if let Some((_, state)) = beliefs.iter().find(|(id, _)| *id == *target) {
                trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                    subject: *target,
                    aspect: BeliefAspect::Hostile,
                    source: state.source,
                    observed_tick: state.observed_tick,
                });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::EngageHostile { target: *target },
            OpportunityAnchor::Entity(*target),
            evidence,
            trace,
        );
    }

    // Remote hostile targets: iterate hostile_targets_of for targets believed
    // at a remote place that satisfy pursuit-profile constraints.
    emit_remote_engage_hostile_targets(
        candidates,
        diagnostics,
        ctx,
        &local_hostiles,
        &raid_targets,
        &current_attackers,
    );
}

fn emit_remote_engage_hostile_targets(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    local_hostiles: &BTreeSet<EntityId>,
    raid_targets: &BTreeSet<EntityId>,
    current_attackers: &BTreeSet<EntityId>,
) {
    let Some(pursuit_profile) = ctx.view.pursuit_profile(ctx.agent) else {
        return;
    };

    let Some(actor_place) = ctx.place else {
        return;
    };

    let policy = ctx.view.belief_confidence_policy(ctx.agent);

    for target in ctx.view.hostile_targets_of(ctx.agent) {
        // Skip targets already handled locally or as raid targets.
        if local_hostiles.contains(&target) || raid_targets.contains(&target) {
            continue;
        }
        if current_attackers.contains(&target) {
            continue;
        }

        let Some(belief) = crate::pursuit_target_belief(ctx.view, ctx.agent, target) else {
            continue;
        };

        let staleness = ctx
            .current_tick
            .0
            .saturating_sub(belief.observed_tick.0);
        let confidence =
            worldwake_core::belief_confidence(&belief.source, staleness, &policy);
        if confidence < pursuit_profile.min_location_confidence {
            continue;
        }

        let Some(route_cost) =
            min_travel_ticks_via_view(ctx.view, actor_place, belief.believed_place)
        else {
            continue;
        };
        if route_cost > pursuit_profile.max_pursuit_travel_ticks.get() {
            continue;
        }

        let goal_key = GoalKey::from(GoalKind::EngageHostile { target });
        if ctx.blocked.is_blocked(
            &goal_key,
            Some(belief.believed_place),
            Some(target),
            None,
            ctx.current_tick,
        ) {
            continue;
        }

        let mut evidence = Evidence::with_entity(target);
        evidence.places.insert(belief.believed_place);
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::EngageHostile { target },
            OpportunityAnchor::Entity(target),
            evidence,
            EvidenceTrace::default(),
        );
    }
}

fn emit_raid_target_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    if is_bandit_raid_deterred_by_wounds(ctx.view, ctx.agent) {
        return;
    }

    if ctx
        .view
        .drive_thresholds(ctx.agent)
        .is_some_and(|thresholds| {
            derive_danger_pressure(ctx.view, ctx.agent) >= thresholds.danger.high()
        })
    {
        return;
    }

    let current_attackers = ctx
        .view
        .current_attackers_of(ctx.agent)
        .into_iter()
        .collect::<BTreeSet<_>>();

    let local_targets = local_raid_targets(ctx.view, ctx.agent, ctx.place)
        .into_iter()
        .collect::<BTreeSet<_>>();

    for target in &local_targets {
        if current_attackers.contains(target) {
            continue;
        }

        let mut evidence = Evidence::with_entity(*target);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::RaidTarget { target: *target },
            OpportunityAnchor::Entity(*target),
            evidence,
            EvidenceTrace::default(),
        );
    }

    // Remote raid targets: iterate entity beliefs for targets believed at a
    // remote place that satisfy pursuit-profile constraints.
    emit_remote_raid_targets(candidates, diagnostics, ctx, &local_targets, &current_attackers);
}

fn emit_remote_raid_targets(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    local_targets: &BTreeSet<EntityId>,
    current_attackers: &BTreeSet<EntityId>,
) {
    let Some(pursuit_profile) = ctx.view.pursuit_profile(ctx.agent) else {
        return;
    };

    let bandit_factions = ctx
        .view
        .bandit_factions_of(ctx.agent)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if bandit_factions.is_empty() {
        return;
    }

    let Some(actor_place) = ctx.place else {
        return;
    };

    let policy = ctx.view.belief_confidence_policy(ctx.agent);

    for (target, _state) in ctx.view.known_entity_beliefs(ctx.agent) {
        // Skip entities already handled as local raid targets.
        if local_targets.contains(&target) {
            continue;
        }
        if current_attackers.contains(&target) {
            continue;
        }
        // Must be an agent.
        if ctx.view.entity_kind(target) != Some(EntityKind::Agent) {
            continue;
        }
        // Must not be in a bandit faction shared with the actor.
        let target_in_bandit_faction = ctx
            .view
            .factions_of(target)
            .into_iter()
            .any(|f| bandit_factions.contains(&f));
        if target_in_bandit_faction {
            continue;
        }

        let Some(belief) = crate::pursuit_target_belief(ctx.view, ctx.agent, target) else {
            continue;
        };

        let staleness = ctx
            .current_tick
            .0
            .saturating_sub(belief.observed_tick.0);
        let confidence =
            worldwake_core::belief_confidence(&belief.source, staleness, &policy);
        if confidence < pursuit_profile.min_location_confidence {
            continue;
        }

        let Some(route_cost) =
            min_travel_ticks_via_view(ctx.view, actor_place, belief.believed_place)
        else {
            continue;
        };
        if route_cost > pursuit_profile.max_pursuit_travel_ticks.get() {
            continue;
        }

        // Check blocked intent for this target/place combination.
        let goal_key = GoalKey::from(GoalKind::RaidTarget { target });
        if ctx.blocked.is_blocked(
            &goal_key,
            Some(belief.believed_place),
            Some(target),
            None,
            ctx.current_tick,
        ) {
            continue;
        }

        let mut evidence = Evidence::with_entity(target);
        evidence.places.insert(belief.believed_place);
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::RaidTarget { target },
            OpportunityAnchor::Entity(target),
            evidence,
            EvidenceTrace::default(),
        );
    }
}

fn emit_self_consume_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    emit_need_driven_candidates(
        candidates,
        diagnostics,
        ctx,
        HomeostaticNeedId::Hunger,
        needs.hunger,
        thresholds.hunger.low(),
        relieves_hunger,
    );
    emit_need_driven_candidates(
        candidates,
        diagnostics,
        ctx,
        HomeostaticNeedId::Thirst,
        needs.thirst,
        thresholds.thirst.low(),
        relieves_thirst,
    );
}

fn emit_need_driven_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    need_id: HomeostaticNeedId,
    current_need: worldwake_core::Permille,
    low_threshold: worldwake_core::Permille,
    matches_need: fn(CommodityKind) -> bool,
) {
    if current_need < low_threshold {
        return;
    }

    // Merchants should not treat their sale stock as personal food/drink.
    let sale_kinds = ctx
        .view
        .merchandise_profile(ctx.agent)
        .map(|p| p.sale_kinds)
        .unwrap_or_default();

    let already_satisfied = CommodityKind::ALL.into_iter().any(|commodity| {
        matches_need(commodity)
            && !sale_kinds.contains(&commodity)
            && local_controlled_commodity_exists(ctx.view, ctx.agent, ctx.place, commodity)
    });

    for commodity in CommodityKind::ALL
        .into_iter()
        .filter(|commodity| matches_need(*commodity))
    {
        // Skip ConsumeOwnedCommodity for merchandise stock — merchants
        // should not eat their own sale inventory.
        if !sale_kinds.contains(&commodity) {
            if let Some(evidence) =
                local_controlled_commodity_evidence(ctx.view, ctx.agent, ctx.place, commodity)
            {
                emit_candidate(
                    candidates,
                    GoalKind::ConsumeOwnedCommodity { commodity },
                    OpportunityAnchor::None,
                    evidence,
                    ctx.blocked,
                    ctx.current_tick,
                );
                continue;
            }
        }

        if already_satisfied {
            continue;
        }

        for (candidate_place, evidence, mut evidence_trace) in direct_acquisition_path_opportunities(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) {
            if ctx.tracing_enabled {
                evidence_trace.knowledge_path.self_knowledge.push(
                    SelfKnowledgeProvenance::NeedLevel {
                        need: need_id,
                        permille: current_need,
                    },
                );
                evidence_trace.knowledge_path.entity_beliefs.extend(
                    belief_provenance_for_contributors(
                        ctx.view,
                        ctx.agent,
                        &evidence_trace.contributors,
                        commodity,
                    ),
                );
            }
            emit_candidate_with_trace(
                candidates,
                diagnostics,
                GoalKind::AcquireCommodity {
                    commodity,
                    purpose: CommodityPurpose::SelfConsume,
                },
                OpportunityAnchor::Place(candidate_place),
                evidence,
                evidence_trace,
            );
        }
    }
}

fn emit_sleep_goal(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    if needs.fatigue >= thresholds.fatigue.low() {
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            trace
                .knowledge_path
                .self_knowledge
                .push(SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Fatigue,
                    permille: needs.fatigue,
                });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::Sleep,
            OpportunityAnchor::None,
            Evidence::with_entity(ctx.agent),
            trace,
        );
    }
}

fn emit_relieve_goal(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    if needs.bladder >= thresholds.bladder.low() {
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            trace
                .knowledge_path
                .self_knowledge
                .push(SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Bladder,
                    permille: needs.bladder,
                });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::Relieve,
            OpportunityAnchor::None,
            Evidence::with_entity(ctx.agent),
            trace,
        );
    }
}

fn emit_wash_goal(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    if needs.dirtiness < thresholds.dirtiness.low() {
        return;
    }

    if let Some(evidence) =
        local_controlled_commodity_evidence(ctx.view, ctx.agent, ctx.place, CommodityKind::Water)
    {
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            trace
                .knowledge_path
                .self_knowledge
                .push(SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Dirtiness,
                    permille: needs.dirtiness,
                });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::Wash,
            OpportunityAnchor::None,
            evidence,
            trace,
        );
    }
}

fn emit_reduce_danger_goal(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(thresholds) = ctx.view.drive_thresholds(ctx.agent) else {
        return;
    };
    let danger_pressure = derive_danger_pressure(ctx.view, ctx.agent);
    if danger_pressure < thresholds.danger.high() {
        return;
    }

    let mut evidence = Evidence::default();
    if let Some(place) = ctx.place {
        let adjacent = ctx.view.adjacent_places_with_travel_ticks(place);
        if !adjacent.is_empty() {
            evidence.places.insert(place);
            evidence.places.extend(
                adjacent
                    .into_iter()
                    .map(|(adjacent_place, _)| adjacent_place),
            );
        }
    }
    if ctx
        .view
        .commodity_quantity(ctx.agent, CommodityKind::Medicine)
        > Quantity(0)
    {
        evidence
            .entities
            .extend(local_wounded_targets(ctx.view, ctx.agent, ctx.place));
    }
    evidence
        .entities
        .extend(ctx.view.current_attackers_of(ctx.agent));

    if !evidence.is_empty() {
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            let wound_count = ctx.view.wounds(ctx.agent).len() as u16;
            trace
                .knowledge_path
                .self_knowledge
                .push(SelfKnowledgeProvenance::OwnWounds { count: wound_count });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::ReduceDanger,
            ctx.place
                .map_or(OpportunityAnchor::None, OpportunityAnchor::Place),
            evidence,
            trace,
        );
    }
}

fn emit_care_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    // Self-care: emit if agent believes self wounded (no medicine gate).
    if ctx.view.has_wounds(ctx.agent) {
        let mut evidence = Evidence::with_entity(ctx.agent);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            let wound_count = ctx.view.wounds(ctx.agent).len() as u16;
            trace
                .knowledge_path
                .self_knowledge
                .push(SelfKnowledgeProvenance::OwnWounds { count: wound_count });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::TreatWounds { patient: ctx.agent },
            OpportunityAnchor::None,
            evidence,
            trace,
        );
    }

    // Third-party care: only for directly-observed wounded others.
    for (entity, belief) in ctx.view.known_entity_beliefs(ctx.agent) {
        if entity == ctx.agent {
            continue;
        }
        if !matches!(belief.source, PerceptionSource::DirectObservation) {
            continue;
        }
        if belief.wounds.is_empty() || !belief.alive {
            continue;
        }
        let mut evidence = Evidence::with_entity(entity);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                subject: entity,
                aspect: BeliefAspect::Wounded,
                source: belief.source,
                observed_tick: belief.observed_tick,
            });
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::TreatWounds { patient: entity },
            OpportunityAnchor::Entity(entity),
            evidence,
            trace,
        );
    }
}

fn local_hostility_targets(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
) -> Vec<EntityId> {
    let Some(place) = place else {
        return Vec::new();
    };

    view.hostile_targets_of(agent)
        .into_iter()
        .filter(|target| {
            view.entity_kind(*target)
                .is_some_and(|kind| kind == worldwake_core::EntityKind::Agent)
        })
        .filter(|target| view.effective_place(*target) == Some(place))
        .collect()
}

fn local_raid_targets(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
) -> Vec<EntityId> {
    let Some(place) = place else {
        return Vec::new();
    };

    let bandit_factions = view
        .bandit_factions_of(agent)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if bandit_factions.is_empty() {
        return Vec::new();
    }

    view.locally_observed_entities_at(agent, place)
        .into_iter()
        .filter(|target| *target != agent)
        .filter(|target| {
            view.entity_kind(*target)
                .is_some_and(|kind| kind == worldwake_core::EntityKind::Agent)
        })
        .filter(|target| view.is_alive(*target) && !view.is_dead(*target))
        .filter(|target| {
            view.factions_of(*target)
                .into_iter()
                .all(|faction| !bandit_factions.contains(&faction))
        })
        .collect()
}

fn social_listeners_at(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: EntityId,
) -> Vec<EntityId> {
    let mut listeners = view
        .entities_at(place)
        .into_iter()
        .filter(|entity| *entity != agent)
        .filter(|entity| view.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| view.is_alive(*entity) && !view.is_dead(*entity))
        .collect::<Vec<_>>();
    listeners.sort_unstable();
    listeners.dedup();
    listeners
}

fn emit_produce_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
) {
    for recipe_id in ctx.view.known_recipes(ctx.agent) {
        let Some(recipe) = ctx.recipes.get(recipe_id) else {
            continue;
        };

        let serves_self_consume = needs.zip(thresholds).is_some_and(|(needs, thresholds)| {
            recipe.outputs.iter().any(|(commodity, _)| {
                (needs.hunger >= thresholds.hunger.low()
                    && relieves_hunger(*commodity)
                    && !any_local_need_relief(ctx.view, ctx.agent, ctx.place, relieves_hunger))
                    || (needs.thirst >= thresholds.thirst.low()
                        && relieves_thirst(*commodity)
                        && !any_local_need_relief(ctx.view, ctx.agent, ctx.place, relieves_thirst))
            })
        });
        let serves_restock = recipe
            .outputs
            .iter()
            .any(|(commodity, _)| ctx.enterprise.restock_gap(*commodity).is_some());

        if !(serves_self_consume || serves_restock) {
            continue;
        }

        for (candidate_place, mut evidence, mut evidence_trace) in recipe_path_opportunities(
            ctx.view,
            ctx.agent,
            ctx.place,
            recipe,
            ctx.recipes,
            ctx.travel_horizon,
        ) {
            if let Some(place) = ctx.place {
                evidence.places.insert(place);
            }
            if ctx.tracing_enabled {
                let primary_commodity = recipe.outputs.first().map(|(c, _)| *c);
                if let Some(commodity) = primary_commodity {
                    evidence_trace.knowledge_path.entity_beliefs.extend(
                        belief_provenance_for_contributors(
                            ctx.view,
                            ctx.agent,
                            &evidence_trace.contributors,
                            commodity,
                        ),
                    );
                }
            }
            emit_candidate_with_trace(
                candidates,
                diagnostics,
                GoalKind::ProduceCommodity { recipe_id },
                OpportunityAnchor::Place(candidate_place),
                evidence,
                evidence_trace,
            );
        }
    }
}

fn emit_restock_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(profile) = ctx.view.merchandise_profile(ctx.agent) else {
        return;
    };

    for commodity in profile.sale_kinds {
        if ctx.enterprise.restock_gap(commodity).is_none() {
            continue;
        }
        for (candidate_place, evidence, mut evidence_trace) in acquisition_path_opportunities(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) {
            if ctx.tracing_enabled {
                evidence_trace
                    .knowledge_path
                    .self_knowledge
                    .push(SelfKnowledgeProvenance::MerchantIdentity);
                evidence_trace.knowledge_path.entity_beliefs.extend(
                    belief_provenance_for_contributors(
                        ctx.view,
                        ctx.agent,
                        &evidence_trace.contributors,
                        commodity,
                    ),
                );
            }
            emit_candidate_with_trace(
                candidates,
                diagnostics,
                GoalKind::RestockCommodity { commodity },
                OpportunityAnchor::Place(candidate_place),
                evidence,
                evidence_trace,
            );
        }
    }
}

fn emit_move_cargo_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(profile) = ctx.view.merchandise_profile(ctx.agent) else {
        return;
    };
    let Some(current_place) = ctx.place else {
        return;
    };
    let Some(destination) = profile.home_market else {
        return;
    };
    if current_place == destination {
        return;
    }

    for commodity in profile.sale_kinds {
        let local_lots = ctx
            .view
            .local_controlled_lots_for(ctx.agent, current_place, commodity);
        if local_lots.is_empty() {
            continue;
        }
        if deliverable_quantity(ctx.view, ctx.agent, current_place, destination, commodity)
            == Quantity(0)
        {
            continue;
        }

        let mut evidence = Evidence::with_place(current_place);
        evidence.places.insert(destination);
        evidence.entities.extend(local_lots.iter().copied());
        let mut trace = EvidenceTrace::default();
        for &lot in &local_lots {
            trace.contributor(CandidateEvidenceKind::LooseLot, current_place, lot);
        }
        if ctx.tracing_enabled {
            trace
                .knowledge_path
                .entity_beliefs
                .extend(belief_provenance_for_contributors(
                    ctx.view,
                    ctx.agent,
                    &trace.contributors,
                    commodity,
                ));
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::MoveCargo {
                commodity,
                destination,
            },
            OpportunityAnchor::Place(destination),
            evidence,
            trace,
        );
    }
}

fn deliverable_quantity(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_place: EntityId,
    destination: EntityId,
    commodity: CommodityKind,
) -> Quantity {
    let local_quantity =
        view.controlled_commodity_quantity_at_place(agent, current_place, commodity);
    let Some(restock_gap) = restock_gap_at_destination(view, agent, destination, commodity) else {
        return Quantity(0);
    };
    let Some(carry_capacity) = view.carry_capacity(agent) else {
        return Quantity(0);
    };
    let Some(current_load) = view.load_of_entity(agent) else {
        return Quantity(0);
    };
    let per_unit = load_per_unit(commodity).0;
    let remaining_capacity = carry_capacity.0.saturating_sub(current_load.0);
    let carry_fit = Quantity(remaining_capacity / per_unit);

    Quantity(local_quantity.0.min(restock_gap.0).min(carry_fit.0))
}

fn emit_loot_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(place) = ctx.place else {
        return;
    };

    let beliefs = if ctx.tracing_enabled {
        ctx.view.known_entity_beliefs(ctx.agent)
    } else {
        Vec::new()
    };

    for corpse in ctx.view.corpse_entities_at(place) {
        if !corpse_has_known_loot(ctx.view, corpse) {
            continue;
        }
        let mut evidence = Evidence::with_entity(corpse);
        evidence.places.insert(place);
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            if let Some((_, state)) = beliefs.iter().find(|(id, _)| *id == corpse) {
                trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                    subject: corpse,
                    aspect: BeliefAspect::Dead,
                    source: state.source,
                    observed_tick: state.observed_tick,
                });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::LootCorpse { corpse },
            OpportunityAnchor::Entity(corpse),
            evidence,
            trace,
        );
    }
}

fn corpse_has_known_loot(view: &dyn GoalBeliefView, corpse: EntityId) -> bool {
    if !view.direct_possessions(corpse).is_empty() {
        return true;
    }

    CommodityKind::ALL
        .iter()
        .copied()
        .any(|commodity| corpse_has_known_commodity(view, corpse, commodity))
}

fn emit_bury_goals(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(place) = ctx.place else {
        return;
    };
    let Some(burial_site) = ctx
        .view
        .matching_workstations_at(place, worldwake_core::WorkstationTag::GravePlot)
        .into_iter()
        .next()
    else {
        return;
    };

    let beliefs = if ctx.tracing_enabled {
        ctx.view.known_entity_beliefs(ctx.agent)
    } else {
        Vec::new()
    };

    for corpse in ctx.view.corpse_entities_at(place) {
        let mut evidence = Evidence::with_entity(corpse);
        evidence.entities.insert(burial_site);
        evidence.places.insert(place);
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            if let Some((_, state)) = beliefs.iter().find(|(id, _)| *id == corpse) {
                trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                    subject: corpse,
                    aspect: BeliefAspect::Dead,
                    source: state.source,
                    observed_tick: state.observed_tick,
                });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            },
            OpportunityAnchor::Entity(corpse),
            evidence,
            trace,
        );
    }
}

fn emit_theft_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(place) = ctx.place else {
        return;
    };
    let Some(deterrence) = assess_theft_deterrence(ctx.view, ctx.agent) else {
        return;
    };
    if deterrence.effective_motive == 0 {
        return;
    }

    let locally_observed = ctx.view.locally_observed_entities_at(ctx.agent, place);

    let Some(carry_capacity) = ctx.view.carry_capacity(ctx.agent) else {
        return;
    };
    let Some(current_load) = ctx.view.load_of_entity(ctx.agent) else {
        return;
    };
    let remaining_capacity = carry_capacity.0.saturating_sub(current_load.0);
    let beliefs = if ctx.tracing_enabled {
        ctx.view.known_entity_beliefs(ctx.agent)
    } else {
        Vec::new()
    };

    for item in locally_observed {
        if ctx.view.entity_kind(item) != Some(EntityKind::ItemLot) {
            continue;
        }
        let Some(owner) = ctx.view.believed_owner_of(item) else {
            continue;
        };
        if owner == ctx.agent || ctx.view.can_control(ctx.agent, item) {
            continue;
        }
        if ctx.view.direct_container(item).is_some() || ctx.view.direct_possessor(item).is_some() {
            continue;
        }
        let Some(item_load) = ctx.view.load_of_entity(item) else {
            continue;
        };
        if item_load.0 > remaining_capacity {
            continue;
        }

        let mut evidence = Evidence::with_entity(item);
        evidence.places.insert(place);
        let mut trace = EvidenceTrace::default();
        if ctx.tracing_enabled {
            if let Some((_, state)) = beliefs.iter().find(|(entity, _)| *entity == item) {
                trace.knowledge_path.entity_beliefs.push(BeliefProvenance {
                    subject: item,
                    aspect: BeliefAspect::LocationAt { place },
                    source: state.source,
                    observed_tick: state.observed_tick,
                });
            }
        }
        emit_candidate_with_trace(
            candidates,
            diagnostics,
            GoalKind::StealItem { target_item: item },
            OpportunityAnchor::Entity(item),
            evidence,
            trace,
        );
    }
}

/// Build `BeliefProvenance` entries for evidence-trace contributors by cross-referencing
/// them against `known_entity_beliefs()`. Only called when tracing is enabled.
fn belief_provenance_for_contributors(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    contributors: &BTreeSet<CandidateEvidenceContributor>,
    commodity: CommodityKind,
) -> Vec<BeliefProvenance> {
    let beliefs = view.known_entity_beliefs(agent);
    let mut result = Vec::new();
    for contributor in contributors {
        let aspect = match contributor.kind {
            CandidateEvidenceKind::Seller | CandidateEvidenceKind::LooseLot => {
                BeliefAspect::HasCommodity { commodity }
            }
            CandidateEvidenceKind::ResourceSource => BeliefAspect::IsResourceSource { commodity },
            CandidateEvidenceKind::RecipeWorkstation => {
                if let Some(tag) = view.workstation_tag(contributor.entity) {
                    BeliefAspect::HasWorkstation { tag }
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if let Some((_, state)) = beliefs.iter().find(|(id, _)| *id == contributor.entity) {
            result.push(BeliefProvenance {
                subject: contributor.entity,
                aspect,
                source: state.source,
                observed_tick: state.observed_tick,
            });
        }
    }
    result
}

fn emit_candidate(
    candidates: &mut Vec<GroundedGoal>,
    kind: GoalKind,
    anchor: OpportunityAnchor,
    evidence: Evidence,
    _blocked: &BlockedIntentMemory,
    _current_tick: Tick,
) {
    if evidence.is_empty() {
        return;
    }

    let key = GoalKey::from(kind);
    candidates.push(GroundedGoal {
        key,
        anchor,
        evidence_entities: evidence.entities,
        evidence_places: evidence.places,
    });
}

fn emit_recorded_violation_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    if ctx.view.violation_disposition_profile(ctx.agent).is_none() {
        return;
    }

    let beliefs = ctx.view.known_entity_beliefs(ctx.agent);
    for record in ctx.violation_memory.unresolved_records(ctx.current_tick) {
        emit_violation_goal(
            candidates,
            diagnostics,
            &beliefs,
            record.id,
            &record.kind,
            ctx,
        );
    }
}

/// Detect expectation violations by comparing stale beliefs against current
/// perception at the agent's current location.  Returns pending violation
/// records for the caller to apply to [`ViolationMemory`].
fn emit_expectation_violation_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) -> Vec<PendingViolationRecord> {
    let mut pending = Vec::new();
    let mut next_violation_id = ctx.violation_memory.next_violation_id();

    // Early return: agent must have a current place (not in transit).
    let Some(current_place) = ctx.place else {
        return pending;
    };

    // Early return: agent must have a ViolationDispositionProfile.
    let Some(profile) = ctx.view.violation_disposition_profile(ctx.agent) else {
        return pending;
    };

    let beliefs = ctx.view.known_entity_beliefs(ctx.agent);
    let observed_at_place: BTreeSet<EntityId> = ctx
        .view
        .locally_observed_entities_at(ctx.agent, current_place)
        .into_iter()
        .collect();

    // Collect violations from belief-perception comparison.
    let mut violations: Vec<(ViolationKind, bool)> = Vec::new();

    for (entity_id, believed_state) in &beliefs {
        // Skip self.
        if *entity_id == ctx.agent {
            continue;
        }

        // Check for EntityMissing: believed at current place, not observed.
        // Exclude in-transit entities: if effective_place is None, the entity
        // is on a travel edge and temporarily absent, not missing.
        if believed_state.last_known_place == Some(current_place)
            && !observed_at_place.contains(entity_id)
            && ctx.view.effective_place(*entity_id).is_some()
        {
            violations.push((
                ViolationKind::EntityMissing {
                    entity: *entity_id,
                    expected_place: current_place,
                },
                true, // emits goal
            ));
        }

        // Check for EntityDead: believed alive, now dead, at current place.
        if believed_state.last_known_place == Some(current_place)
            && believed_state.alive
            && observed_at_place.contains(entity_id)
            && ctx.view.locally_observed_is_dead(ctx.agent, *entity_id)
        {
            violations.push((
                ViolationKind::EntityDead { entity: *entity_id },
                false, // record only, no goal
            ));
        }

        // Check for SupplyDepleted: believed resource source at current place
        // with available quantity > 0, now observed at 0.
        if let Some(resource_source) = &believed_state.resource_source {
            if believed_state.last_known_place == Some(current_place)
                && resource_source.available_quantity > Quantity(0)
                && ctx.view.locally_observed_commodity_quantity(
                    ctx.agent,
                    *entity_id,
                    resource_source.commodity,
                ) == Quantity(0)
            {
                violations.push((
                    ViolationKind::SupplyDepleted {
                        commodity: resource_source.commodity,
                        source: *entity_id,
                        place: current_place,
                    },
                    true, // emits goal
                ));
            }
        }
    }

    let ttl = profile.violation_memory_retention_ticks;

    for (kind, emits_goal) in violations {
        // Skip already-recorded (unexpired) violations.
        if ctx.violation_memory.is_recorded(&kind, ctx.current_tick) {
            continue;
        }
        if pending
            .iter()
            .any(|record: &PendingViolationRecord| record.kind == kind)
        {
            continue;
        }

        // Always record the violation for future suppression.
        let violation_id = next_violation_id;
        next_violation_id = ViolationId(next_violation_id.0 + 1);
        pending.push(PendingViolationRecord {
            id: violation_id,
            kind: kind.clone(),
            observed_tick: ctx.current_tick,
            ttl,
        });

        if emits_goal {
            emit_violation_goal(candidates, diagnostics, &beliefs, violation_id, &kind, ctx);
        }
    }

    pending
}

/// Emit an `InvestigateViolation` goal candidate for an `EntityMissing` or
/// `SupplyDepleted` violation, with belief-observation contradiction provenance.
fn emit_violation_goal(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    beliefs: &[(EntityId, BelievedEntityState)],
    violation_id: ViolationId,
    kind: &ViolationKind,
    ctx: &GenerationContext<'_>,
) {
    let (investigation_place, entity_id) = match kind {
        ViolationKind::EntityMissing {
            entity,
            expected_place,
        } => (*expected_place, *entity),
        ViolationKind::SupplyDepleted { source, place, .. } => (*place, *source),
        ViolationKind::EntityDead { .. } | ViolationKind::SuspectedTheft { .. } => return,
    };

    let belief_entry = beliefs.iter().find(|(id, _)| *id == entity_id);
    let (source, observed_tick) = belief_entry.map_or(
        (PerceptionSource::DirectObservation, ctx.current_tick),
        |(_, b)| (b.source, b.observed_tick),
    );

    let aspect = match kind {
        ViolationKind::EntityMissing { expected_place, .. } => BeliefAspect::LocationAt {
            place: *expected_place,
        },
        ViolationKind::SupplyDepleted { commodity, .. } => BeliefAspect::HasCommodity {
            commodity: *commodity,
        },
        ViolationKind::EntityDead { .. } | ViolationKind::SuspectedTheft { .. } => return,
    };

    let trace = EvidenceTrace {
        contributors: BTreeSet::new(),
        exclusions: BTreeSet::new(),
        knowledge_path: KnowledgePath {
            self_knowledge: Vec::new(),
            entity_beliefs: vec![BeliefProvenance {
                subject: entity_id,
                aspect,
                source,
                observed_tick,
            }],
            institutional_beliefs: Vec::new(),
        },
        legality: None,
    };

    let evidence = Evidence {
        entities: BTreeSet::from([entity_id]),
        places: BTreeSet::from([investigation_place]),
    };

    emit_candidate_with_trace(
        candidates,
        diagnostics,
        GoalKind::InvestigateViolation {
            violation_id,
            place: investigation_place,
        },
        OpportunityAnchor::Place(investigation_place),
        evidence,
        trace,
    );
}

fn emit_candidate_with_trace(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    kind: GoalKind,
    anchor: OpportunityAnchor,
    evidence: Evidence,
    evidence_trace: EvidenceTrace,
) {
    if evidence.is_empty() {
        return;
    }

    let key = GoalKey::from(kind);
    let opportunity = OpportunityKey {
        goal_key: key,
        anchor,
    };
    candidates.push(GroundedGoal {
        key,
        anchor,
        evidence_entities: evidence.entities,
        evidence_places: evidence.places,
    });

    let trace = evidence_trace.into_public(opportunity);
    diagnostics
        .evidence
        .entry(opportunity)
        .and_modify(|existing| merge_candidate_evidence_trace(existing, &trace))
        .or_insert(trace);
}

fn merge_candidate_evidence_trace(
    existing: &mut CandidateEvidenceTrace,
    incoming: &CandidateEvidenceTrace,
) {
    for contributor in &incoming.contributors {
        if !existing.contributors.contains(contributor) {
            existing.contributors.push(*contributor);
        }
    }
    for exclusion in &incoming.exclusions {
        if !existing.exclusions.contains(exclusion) {
            existing.exclusions.push(*exclusion);
        }
    }
    existing.contributors.sort();
    existing.exclusions.sort();
    existing
        .knowledge_path
        .entity_beliefs
        .extend(incoming.knowledge_path.entity_beliefs.iter().cloned());
    existing
        .knowledge_path
        .self_knowledge
        .extend(incoming.knowledge_path.self_knowledge.iter().cloned());
    existing.knowledge_path.institutional_beliefs.extend(
        incoming
            .knowledge_path
            .institutional_beliefs
            .iter()
            .cloned(),
    );
    if existing.legality.is_none() {
        existing.legality.clone_from(&incoming.legality);
    } else {
        debug_assert!(
            incoming.legality.is_none() || existing.legality == incoming.legality,
            "candidate legality provenance diverged for one grounded goal"
        );
    }
}

fn acquisition_path_opportunities(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
) -> Vec<(EntityId, Evidence, EvidenceTrace)> {
    acquisition_path_opportunities_inner(
        view,
        agent,
        place,
        commodity,
        recipes,
        travel_horizon,
        AcquisitionSearchOptions {
            include_recipes: true,
            visited_commodities: &BTreeSet::new(),
        },
    )
}

fn direct_acquisition_path_opportunities(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
) -> Vec<(EntityId, Evidence, EvidenceTrace)> {
    acquisition_path_opportunities_inner(
        view,
        agent,
        place,
        commodity,
        recipes,
        travel_horizon,
        AcquisitionSearchOptions {
            include_recipes: false,
            visited_commodities: &BTreeSet::new(),
        },
    )
}

fn acquisition_path_opportunities_inner(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
    options: AcquisitionSearchOptions<'_>,
) -> Vec<(EntityId, Evidence, EvidenceTrace)> {
    let Some(origin) = place else {
        return Vec::new();
    };

    reachable_places_within_horizon(view, origin, travel_horizon)
        .into_iter()
        .filter_map(|candidate_place| {
            acquisition_path_evidence_at_place(
                view,
                agent,
                candidate_place,
                commodity,
                recipes,
                travel_horizon,
                options,
            )
            .map(|(evidence, trace)| (candidate_place, evidence, trace))
        })
        .collect()
}

fn acquisition_path_evidence_inner(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
    options: AcquisitionSearchOptions<'_>,
) -> Option<(Evidence, EvidenceTrace)> {
    let place = place?;
    let mut visited_commodities = options.visited_commodities.clone();
    if !visited_commodities.insert(commodity) {
        return None;
    }
    let mut evidence = Evidence::with_place(place);
    let mut trace = EvidenceTrace::default();

    for candidate_place in reachable_places_within_horizon(view, place, travel_horizon) {
        let mut place_evidence = Evidence::default();
        let mut place_trace = EvidenceTrace::default();

        for seller in view.agents_selling_at(candidate_place, commodity) {
            if seller != agent {
                place_evidence.places.insert(candidate_place);
                place_evidence.entities.insert(seller);
                place_trace.contributor(CandidateEvidenceKind::Seller, candidate_place, seller);
            }
        }
        if let Some(local_lots) =
            local_unpossessed_commodity_evidence(view, candidate_place, commodity)
        {
            for lot in &local_lots.entities {
                place_trace.contributor(CandidateEvidenceKind::LooseLot, candidate_place, *lot);
            }
            place_evidence.merge(local_lots);
        }
        for source in view.resource_sources_at(candidate_place, commodity) {
            if view
                .resource_source(source)
                .is_some_and(|resource| resource.available_quantity > Quantity(0))
            {
                place_evidence.places.insert(candidate_place);
                place_evidence.entities.insert(source);
                place_trace.contributor(
                    CandidateEvidenceKind::ResourceSource,
                    candidate_place,
                    source,
                );
            } else {
                place_trace.exclusion(
                    CandidateEvidenceKind::ResourceSource,
                    candidate_place,
                    source,
                    CandidateEvidenceExclusionReason::DepletedResourceSource,
                );
            }
        }
        for corpse in view.corpse_entities_at(candidate_place) {
            if corpse_contains_commodity(view, corpse, commodity) {
                place_evidence.places.insert(candidate_place);
                place_evidence.entities.insert(corpse);
                place_trace.contributor(CandidateEvidenceKind::Corpse, candidate_place, corpse);
            }
        }
        if options.include_recipes {
            for recipe_id in view.known_recipes(agent) {
                let Some(recipe) = recipes.get(recipe_id) else {
                    continue;
                };
                if !recipe
                    .outputs
                    .iter()
                    .any(|(output, _)| *output == commodity)
                {
                    continue;
                }
                if let Some((recipe_evidence, recipe_trace)) = recipe_path_evidence_inner(
                    view,
                    agent,
                    Some(candidate_place),
                    recipe,
                    recipes,
                    travel_horizon,
                    &visited_commodities,
                ) {
                    place_evidence.merge(recipe_evidence);
                    place_trace.merge(recipe_trace);
                }
            }
        }

        if !place_evidence.is_empty() {
            evidence.merge(place_evidence);
        }
        if !place_trace.is_empty() {
            trace.merge(place_trace);
        }
    }

    (!evidence.entities.is_empty()).then_some((evidence, trace))
}

fn acquisition_path_evidence_at_place(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    candidate_place: EntityId,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
    options: AcquisitionSearchOptions<'_>,
) -> Option<(Evidence, EvidenceTrace)> {
    let mut place_evidence = Evidence::default();
    let mut place_trace = EvidenceTrace::default();

    for seller in view.agents_selling_at(candidate_place, commodity) {
        if seller != agent {
            place_evidence.places.insert(candidate_place);
            place_evidence.entities.insert(seller);
            place_trace.contributor(CandidateEvidenceKind::Seller, candidate_place, seller);
        }
    }
    if let Some(local_lots) = local_unpossessed_commodity_evidence(view, candidate_place, commodity)
    {
        for lot in &local_lots.entities {
            place_trace.contributor(CandidateEvidenceKind::LooseLot, candidate_place, *lot);
        }
        place_evidence.merge(local_lots);
    }
    for source in view.resource_sources_at(candidate_place, commodity) {
        if view
            .resource_source(source)
            .is_some_and(|resource| resource.available_quantity > Quantity(0))
        {
            place_evidence.places.insert(candidate_place);
            place_evidence.entities.insert(source);
            place_trace.contributor(
                CandidateEvidenceKind::ResourceSource,
                candidate_place,
                source,
            );
        } else {
            place_trace.exclusion(
                CandidateEvidenceKind::ResourceSource,
                candidate_place,
                source,
                CandidateEvidenceExclusionReason::DepletedResourceSource,
            );
        }
    }
    for corpse in view.corpse_entities_at(candidate_place) {
        if corpse_contains_commodity(view, corpse, commodity) {
            place_evidence.places.insert(candidate_place);
            place_evidence.entities.insert(corpse);
            place_trace.contributor(CandidateEvidenceKind::Corpse, candidate_place, corpse);
        }
    }
    if options.include_recipes {
        for recipe_id in view.known_recipes(agent) {
            let Some(recipe) = recipes.get(recipe_id) else {
                continue;
            };
            if !recipe
                .outputs
                .iter()
                .any(|(output, _)| *output == commodity)
            {
                continue;
            }
            if let Some((recipe_evidence, recipe_trace)) = recipe_path_evidence_at_place(
                view,
                agent,
                candidate_place,
                recipe,
                recipes,
                travel_horizon,
                options.visited_commodities,
            ) {
                place_evidence.merge(recipe_evidence);
                place_trace.merge(recipe_trace);
            }
        }
    }

    (!place_evidence.is_empty()).then_some((place_evidence, place_trace))
}

fn reachable_places_within_horizon(
    view: &dyn GoalBeliefView,
    origin: EntityId,
    travel_horizon: u8,
) -> Vec<EntityId> {
    let mut ordered = vec![origin];
    let mut visited = BTreeSet::from([origin]);
    let mut frontier = VecDeque::from([(origin, 0u8)]);

    while let Some((place, depth)) = frontier.pop_front() {
        if depth >= travel_horizon {
            continue;
        }
        for (adjacent, _) in view.adjacent_places_with_travel_ticks(place) {
            if visited.insert(adjacent) {
                ordered.push(adjacent);
                frontier.push_back((adjacent, depth.saturating_add(1)));
            }
        }
    }

    ordered
}

/// Compute minimum travel ticks from `from` to `to` using BFS over the
/// belief view's adjacency graph.  Returns `None` if unreachable.
fn min_travel_ticks_via_view(
    view: &dyn GoalBeliefView,
    from: EntityId,
    to: EntityId,
) -> Option<u32> {
    if from == to {
        return Some(0);
    }
    let mut visited = BTreeMap::new();
    let mut heap = std::collections::BinaryHeap::new();
    visited.insert(from, 0u32);
    heap.push(std::cmp::Reverse((0u32, from)));
    while let Some(std::cmp::Reverse((cost, place))) = heap.pop() {
        if place == to {
            return Some(cost);
        }
        if cost > *visited.get(&place).unwrap_or(&u32::MAX) {
            continue;
        }
        for (adj, ticks) in view.adjacent_places_with_travel_ticks(place) {
            let next_cost = cost.saturating_add(ticks.get());
            if next_cost < *visited.get(&adj).unwrap_or(&u32::MAX) {
                visited.insert(adj, next_cost);
                heap.push(std::cmp::Reverse((next_cost, adj)));
            }
        }
    }
    None
}

fn local_unpossessed_commodity_evidence(
    view: &dyn GoalBeliefView,
    place: EntityId,
    commodity: CommodityKind,
) -> Option<Evidence> {
    let mut evidence = Evidence::with_place(place);
    for entity in view.entities_at(place) {
        if view.item_lot_commodity(entity) != Some(commodity) {
            continue;
        }
        if view.direct_container(entity).is_some() || view.direct_possessor(entity).is_some() {
            continue;
        }
        evidence.entities.insert(entity);
    }
    (!evidence.entities.is_empty()).then_some(evidence)
}

fn recipe_path_evidence_inner(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    recipe: &RecipeDefinition,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
    visited_commodities: &BTreeSet<CommodityKind>,
) -> Option<(Evidence, EvidenceTrace)> {
    let place = place?;
    let (workstation_evidence, workstation_trace) =
        available_recipe_workstation_evidence(view, agent, Some(place), recipe, travel_horizon)?;

    if recipe.inputs.is_empty() {
        let mut evidence = Evidence::with_place(place);
        let mut trace = workstation_trace;
        for workstation in &workstation_evidence.entities {
            let &(output_commodity, output_quantity) = recipe.outputs.first()?;
            let source_ok = view.resource_source(*workstation).is_some_and(|source| {
                source.commodity == output_commodity && source.available_quantity >= output_quantity
            });
            if source_ok {
                evidence.entities.insert(*workstation);
                trace.contributor(CandidateEvidenceKind::ResourceSource, place, *workstation);
            }
        }
        return (!evidence.entities.is_empty()).then_some((evidence, trace));
    }

    let mut evidence = workstation_evidence;
    let mut trace = workstation_trace;
    for (commodity, required_quantity) in aggregate_recipe_quantities(&recipe.inputs) {
        let owned_quantity = view.commodity_quantity(agent, commodity);
        if owned_quantity >= required_quantity {
            continue;
        }

        let (input_evidence, input_trace) = acquisition_path_evidence_inner(
            view,
            agent,
            Some(place),
            commodity,
            recipes,
            travel_horizon,
            AcquisitionSearchOptions {
                include_recipes: true,
                visited_commodities,
            },
        )?;
        evidence.merge(input_evidence);
        trace.merge(input_trace);
    }

    Some((evidence, trace))
}

fn recipe_path_opportunities(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    recipe: &RecipeDefinition,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
) -> Vec<(EntityId, Evidence, EvidenceTrace)> {
    let Some(origin) = place else {
        return Vec::new();
    };

    reachable_places_within_horizon(view, origin, travel_horizon)
        .into_iter()
        .filter_map(|candidate_place| {
            recipe_path_evidence_at_place(
                view,
                agent,
                candidate_place,
                recipe,
                recipes,
                travel_horizon,
                &BTreeSet::new(),
            )
            .map(|(evidence, trace)| (candidate_place, evidence, trace))
        })
        .collect()
}

fn recipe_path_evidence_at_place(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: EntityId,
    recipe: &RecipeDefinition,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
    visited_commodities: &BTreeSet<CommodityKind>,
) -> Option<(Evidence, EvidenceTrace)> {
    let (workstation_evidence, workstation_trace) =
        available_recipe_workstation_evidence_at_place(view, agent, place, recipe)?;

    if recipe.inputs.is_empty() {
        let mut evidence = Evidence::with_place(place);
        let mut trace = workstation_trace;
        for workstation in &workstation_evidence.entities {
            let &(output_commodity, output_quantity) = recipe.outputs.first()?;
            let source_ok = view.resource_source(*workstation).is_some_and(|source| {
                source.commodity == output_commodity && source.available_quantity >= output_quantity
            });
            if source_ok {
                evidence.entities.insert(*workstation);
                trace.contributor(CandidateEvidenceKind::ResourceSource, place, *workstation);
            }
        }
        return (!evidence.entities.is_empty()).then_some((evidence, trace));
    }

    let mut evidence = workstation_evidence;
    let mut trace = workstation_trace;
    for (commodity, required_quantity) in aggregate_recipe_quantities(&recipe.inputs) {
        let owned_quantity = view.commodity_quantity(agent, commodity);
        if owned_quantity >= required_quantity {
            continue;
        }

        let (input_evidence, input_trace) = acquisition_path_evidence_inner(
            view,
            agent,
            Some(place),
            commodity,
            recipes,
            travel_horizon,
            AcquisitionSearchOptions {
                include_recipes: true,
                visited_commodities,
            },
        )?;
        evidence.merge(input_evidence);
        trace.merge(input_trace);
    }

    Some((evidence, trace))
}

fn available_recipe_workstation_evidence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    recipe: &RecipeDefinition,
    travel_horizon: u8,
) -> Option<(Evidence, EvidenceTrace)> {
    let place = place?;
    let workstation_tag = recipe.required_workstation_tag?;

    for required_tool in &recipe.required_tool_kinds {
        if view.unique_item_count(agent, *required_tool) == 0 {
            return None;
        }
    }

    let mut evidence = Evidence::default();
    let mut trace = EvidenceTrace::default();
    for candidate_place in reachable_places_within_horizon(view, place, travel_horizon) {
        let available_workstations = view
            .matching_workstations_at(candidate_place, workstation_tag)
            .into_iter()
            .filter(|workstation| !view.has_production_job(*workstation))
            .collect::<Vec<_>>();
        if available_workstations.is_empty() {
            continue;
        }
        evidence.places.insert(candidate_place);
        for workstation in available_workstations {
            evidence.entities.insert(workstation);
            trace.contributor(
                CandidateEvidenceKind::RecipeWorkstation,
                candidate_place,
                workstation,
            );
        }
    }
    (!evidence.entities.is_empty()).then_some((evidence, trace))
}

fn available_recipe_workstation_evidence_at_place(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: EntityId,
    recipe: &RecipeDefinition,
) -> Option<(Evidence, EvidenceTrace)> {
    let workstation_tag = recipe.required_workstation_tag?;

    for required_tool in &recipe.required_tool_kinds {
        if view.unique_item_count(agent, *required_tool) == 0 {
            return None;
        }
    }

    let mut evidence = Evidence::default();
    let mut trace = EvidenceTrace::default();
    let available_workstations = view
        .matching_workstations_at(place, workstation_tag)
        .into_iter()
        .filter(|workstation| !view.has_production_job(*workstation))
        .collect::<Vec<_>>();
    if available_workstations.is_empty() {
        return None;
    }
    evidence.places.insert(place);
    for workstation in available_workstations {
        evidence.entities.insert(workstation);
        trace.contributor(CandidateEvidenceKind::RecipeWorkstation, place, workstation);
    }
    Some((evidence, trace))
}

fn aggregate_recipe_quantities(
    entries: &[(CommodityKind, Quantity)],
) -> BTreeMap<CommodityKind, Quantity> {
    let mut aggregated = BTreeMap::new();
    for (commodity, quantity) in entries {
        aggregated
            .entry(*commodity)
            .and_modify(|current: &mut Quantity| current.0 += quantity.0)
            .or_insert(*quantity);
    }
    aggregated
}

fn local_wounded_targets(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
) -> Vec<EntityId> {
    let mut targets = BTreeSet::new();
    if view.is_alive(agent) && view.has_wounds(agent) {
        targets.insert(agent);
    }
    if let Some(place) = place {
        for entity in view.entities_at(place) {
            if view.is_alive(entity) && view.has_wounds(entity) {
                targets.insert(entity);
            }
        }
    }
    targets.into_iter().collect()
}

fn local_controlled_commodity_exists(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
) -> bool {
    local_controlled_commodity_evidence(view, agent, place, commodity).is_some()
}

fn local_controlled_commodity_evidence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
) -> Option<Evidence> {
    let place = place?;
    let mut evidence = Evidence::with_place(place);
    let mut local_entities = BTreeSet::new();
    local_entities.extend(view.entities_at(place));
    local_entities.extend(view.direct_possessions(agent));
    for entity in local_entities {
        if view.item_lot_commodity(entity) != Some(commodity) || !view.can_control(agent, entity) {
            continue;
        }
        evidence.entities.insert(entity);
    }
    (!evidence.entities.is_empty()).then_some(evidence)
}

fn any_local_need_relief(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    matches_need: fn(CommodityKind) -> bool,
) -> bool {
    CommodityKind::ALL.into_iter().any(|commodity| {
        matches_need(commodity)
            && (local_controlled_commodity_exists(view, agent, place, commodity)
                || place
                    .and_then(|place| local_unpossessed_commodity_evidence(view, place, commodity))
                    .is_some())
    })
}

fn corpse_contains_commodity(
    view: &dyn GoalBeliefView,
    corpse: EntityId,
    commodity: CommodityKind,
) -> bool {
    corpse_has_known_commodity(view, corpse, commodity)
}

fn corpse_has_known_commodity(
    view: &dyn GoalBeliefView,
    corpse: EntityId,
    commodity: CommodityKind,
) -> bool {
    view.direct_possessions(corpse)
        .into_iter()
        .any(|entity| view.item_lot_commodity(entity) == Some(commodity))
        || view.commodity_quantity(corpse, commodity) > Quantity(0)
}

fn relieves_hunger(commodity: CommodityKind) -> bool {
    commodity
        .spec()
        .consumable_profile
        .is_some_and(|profile| profile.hunger_relief_per_unit.value() > 0)
}

fn relieves_thirst(commodity: CommodityKind) -> bool {
    commodity
        .spec()
        .consumable_profile
        .is_some_and(|profile| profile.thirst_relief_per_unit.value() > 0)
}

#[cfg(test)]
mod tests {
    use super::{
        deliverable_quantity, emit_produce_goals, emit_restock_goals, generate_candidates,
        generate_candidates_with_travel_horizon, CandidateGenerationDiagnostics, GenerationContext,
    };
    use crate::{
        enterprise::{analyze_candidate_enterprise, EnterpriseSignals},
        knowledge_path::{
            BeliefAspect, InstitutionalBeliefProvenance, KnowledgePath, SelfKnowledgeProvenance,
        },
        BanditCandidateOmission, BanditCandidateOmissionReason, BanditGoalFamily,
        CandidateEvidenceTrace, PoliticalCandidateOmissionReason, PoliticalGoalFamily,
        SocialCandidateOmission,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BelievedEntityState, BelievedInstitutionalClaim, BlockedIntent, BlockedIntentMemory,
        BlockerKey, BlockingFact, BodyPart, CombatProfile, CommodityConsumableProfile,
        CommodityKind, CommodityPurpose, DemandObservation, DemandObservationReason,
        DriveThresholds, EligibilityRule, EntityId, EntityKind, EpistemicDispositionProfile,
        GoalKey, GoalKind, HomeostaticNeedId, HomeostaticNeeds, InTransitOnEdge,
        InstitutionalBeliefKey, InstitutionalBeliefRead, InstitutionalClaim,
        InstitutionalKnowledgeSource, LoadUnits, MerchandiseProfile, MetabolismProfile, OfficeData,
        PatrolProfile, PatrolRoute, PerceptionSource, Permille, PunishmentFineSelectionTrace,
        PunishmentFineTraceFacts, Quantity, RecipeId, RecipientKnowledgeStatus, RecordData,
        RecordEntryId, RecordKind, ResourceSource, SharedTellState, SocialObservation,
        SocialObservationDetail, TellMemoryKey, TellProfile, TellTopic, TheftFacts, Tick,
        TickRange, ToldBeliefMemory, TradeDispositionProfile, UniqueItemKind, ViolationKind,
        ViolationMemory, WorkstationTag, Wound, WoundCause, WoundId, OpportunityAnchor,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, DurationExpr, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView, TellTopicOmissionReason,
    };

    struct TestBeliefView {
        current_tick: Tick,
        alive: BTreeSet<EntityId>,
        dead: BTreeSet<EntityId>,
        incapacitated: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_places: BTreeMap<EntityId, Vec<EntityId>>,
        unique_item_counts: BTreeMap<(EntityId, UniqueItemKind), u32>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        locally_observed_commodity_quantities:
            BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        workstation_tags: BTreeMap<EntityId, WorkstationTag>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        production_jobs: BTreeSet<EntityId>,
        controllable: BTreeSet<(EntityId, EntityId)>,
        controlled_entities: BTreeSet<EntityId>,
        homeostatic_needs: BTreeMap<EntityId, HomeostaticNeeds>,
        drive_thresholds: BTreeMap<EntityId, DriveThresholds>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        courage: BTreeMap<EntityId, Permille>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        sellers: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        known_recipes: BTreeMap<EntityId, Vec<RecipeId>>,
        workstations: BTreeMap<(EntityId, WorkstationTag), Vec<EntityId>>,
        sources_at: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        corpses_at: BTreeMap<EntityId, Vec<EntityId>>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        social_observations: BTreeMap<EntityId, Vec<worldwake_core::SocialObservation>>,
        tell_profiles: BTreeMap<EntityId, TellProfile>,
        told_beliefs: BTreeMap<EntityId, Vec<(TellMemoryKey, ToldBeliefMemory)>>,
        record_data: BTreeMap<EntityId, RecordData>,
        office_data: BTreeMap<EntityId, OfficeData>,
        office_holders: BTreeMap<EntityId, EntityId>,
        office_holder_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        force_controller_beliefs:
            BTreeMap<EntityId, InstitutionalBeliefRead<(Option<EntityId>, bool)>>,
        factions_by_member: BTreeMap<EntityId, Vec<EntityId>>,
        faction_rally_point_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        bandit_factions: BTreeSet<EntityId>,
        bandit_flee_thresholds: BTreeMap<EntityId, Permille>,
        local_bandit_camps: BTreeMap<EntityId, EntityId>,
        loyalties: BTreeMap<(EntityId, EntityId), Permille>,
        support_declarations: BTreeMap<(EntityId, EntityId), EntityId>,
        support_declaration_beliefs:
            BTreeMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
        institutional_claims:
            BTreeMap<(EntityId, InstitutionalBeliefKey), Vec<BelievedInstitutionalClaim>>,
        epistemic_disposition_profiles: BTreeMap<EntityId, EpistemicDispositionProfile>,
        violation_disposition_profiles:
            BTreeMap<EntityId, worldwake_core::ViolationDispositionProfile>,
        theft_disposition_profiles: BTreeMap<EntityId, worldwake_core::TheftDispositionProfile>,
        justice_disposition_profiles: BTreeMap<EntityId, worldwake_core::JusticeDispositionProfile>,
        patrol_profiles: BTreeMap<EntityId, PatrolProfile>,
        patrol_routes: BTreeMap<EntityId, PatrolRoute>,
        pursuit_profiles: BTreeMap<EntityId, worldwake_core::PursuitProfile>,
        reservation_ranges: BTreeMap<EntityId, Vec<TickRange>>,
        in_transit: BTreeSet<EntityId>,
        believed_owners: BTreeMap<EntityId, EntityId>,
    }

    impl Default for TestBeliefView {
        fn default() -> Self {
            Self {
                current_tick: Tick(0),
                alive: BTreeSet::new(),
                dead: BTreeSet::new(),
                incapacitated: BTreeSet::new(),
                entity_kinds: BTreeMap::new(),
                effective_places: BTreeMap::new(),
                entities_at: BTreeMap::new(),
                direct_possessions: BTreeMap::new(),
                adjacent_places: BTreeMap::new(),
                unique_item_counts: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                locally_observed_commodity_quantities: BTreeMap::new(),
                carry_capacities: BTreeMap::new(),
                entity_loads: BTreeMap::new(),
                lot_commodities: BTreeMap::new(),
                consumable_profiles: BTreeMap::new(),
                direct_containers: BTreeMap::new(),
                direct_possessors: BTreeMap::new(),
                workstation_tags: BTreeMap::new(),
                resource_sources: BTreeMap::new(),
                production_jobs: BTreeSet::new(),
                controllable: BTreeSet::new(),
                controlled_entities: BTreeSet::new(),
                homeostatic_needs: BTreeMap::new(),
                drive_thresholds: BTreeMap::new(),
                wounds: BTreeMap::new(),
                courage: BTreeMap::new(),
                hostiles: BTreeMap::new(),
                attackers: BTreeMap::new(),
                sellers: BTreeMap::new(),
                known_recipes: BTreeMap::new(),
                workstations: BTreeMap::new(),
                sources_at: BTreeMap::new(),
                demand_memory: BTreeMap::new(),
                merchandise_profiles: BTreeMap::new(),
                corpses_at: BTreeMap::new(),
                beliefs: BTreeMap::new(),
                social_observations: BTreeMap::new(),
                tell_profiles: BTreeMap::new(),
                told_beliefs: BTreeMap::new(),
                record_data: BTreeMap::new(),
                office_data: BTreeMap::new(),
                office_holders: BTreeMap::new(),
                office_holder_beliefs: BTreeMap::new(),
                force_controller_beliefs: BTreeMap::new(),
                factions_by_member: BTreeMap::new(),
                faction_rally_point_beliefs: BTreeMap::new(),
                bandit_factions: BTreeSet::new(),
                bandit_flee_thresholds: BTreeMap::new(),
                local_bandit_camps: BTreeMap::new(),
                loyalties: BTreeMap::new(),
                support_declarations: BTreeMap::new(),
                support_declaration_beliefs: BTreeMap::new(),
                institutional_claims: BTreeMap::new(),
                epistemic_disposition_profiles: BTreeMap::new(),
                violation_disposition_profiles: BTreeMap::new(),
                theft_disposition_profiles: BTreeMap::new(),
                justice_disposition_profiles: BTreeMap::new(),
                patrol_profiles: BTreeMap::new(),
                patrol_routes: BTreeMap::new(),
                pursuit_profiles: BTreeMap::new(),
                reservation_ranges: BTreeMap::new(),
                in_transit: BTreeSet::new(),
                believed_owners: BTreeMap::new(),
            }
        }
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn current_tick(&self) -> Tick {
            self.current_tick
        }

        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity) && !self.dead.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, entity: EntityId) -> bool {
            self.in_transit.contains(&entity)
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn known_social_observations(&self, agent: EntityId) -> Vec<SocialObservation> {
            self.social_observations
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
            self.institutional_claims
                .iter()
                .filter(|((claim_agent, _), _)| *claim_agent == agent)
                .flat_map(|(_, claims)| claims.iter().cloned())
                .collect()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }

        fn factions_of(&self, entity: EntityId) -> Vec<EntityId> {
            self.factions_by_member
                .get(&entity)
                .cloned()
                .unwrap_or_default()
        }

        fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
            self.factions_of(entity)
                .into_iter()
                .filter(|faction| self.bandit_factions.contains(faction))
                .collect()
        }

        fn locally_observed_bandit_camp_faction_at(
            &self,
            agent: EntityId,
            place: EntityId,
        ) -> Option<EntityId> {
            (self.effective_place(agent) == Some(place))
                .then(|| self.local_bandit_camps.get(&place).copied())
                .flatten()
        }

        fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
            self.known_recipes(actor).contains(&recipe)
        }

        fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
            self.unique_item_counts
                .get(&(holder, kind))
                .copied()
                .unwrap_or(0)
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }
        fn locally_observed_commodity_quantity(
            &self,
            agent: EntityId,
            holder: EntityId,
            kind: CommodityKind,
        ) -> Quantity {
            self.locally_observed_commodity_quantities
                .get(&(agent, holder, kind))
                .copied()
                .unwrap_or_else(|| self.commodity_quantity(holder, kind))
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.local_controlled_lots_for(actor, place, commodity)
                .into_iter()
                .fold(Quantity(0), |total, entity| {
                    let quantity = self
                        .commodity_quantities
                        .get(&(entity, commodity))
                        .copied()
                        .unwrap_or(Quantity(0));
                    Quantity(total.0 + quantity.0)
                })
        }
        fn local_controlled_lots_for(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            let mut entities = self.entities_at(place);
            entities.extend(self.direct_possessions(actor));
            entities.sort();
            entities.dedup();
            entities
                .into_iter()
                .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
                .filter(|entity| self.can_control(actor, *entity))
                .collect()
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            self.consumable_profiles.get(&entity).copied()
        }

        fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_containers.get(&entity).copied()
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
            self.believed_owners.get(&entity).copied()
        }

        fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
            self.workstation_tags.get(&entity).copied()
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, entity: EntityId) -> bool {
            self.production_jobs.contains(&entity)
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            self.controllable.contains(&(actor, entity))
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.controlled_entities.contains(&entity)
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
            self.reservation_ranges
                .get(&entity)
                .cloned()
                .unwrap_or_default()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn is_incapacitated(&self, entity: EntityId) -> bool {
            self.incapacitated.contains(&entity)
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.homeostatic_needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.drive_thresholds.get(&agent).copied()
        }
        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courage.get(&agent).copied()
        }
        fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
            self.bandit_flee_thresholds.get(&faction).copied()
        }
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
            self.patrol_profiles.get(&agent).cloned()
        }

        fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
            self.patrol_routes.get(&agent).cloned()
        }

        fn pursuit_profile(&self, agent: EntityId) -> Option<worldwake_core::PursuitProfile> {
            self.pursuit_profiles.get(&agent).cloned()
        }

        fn epistemic_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<EpistemicDispositionProfile> {
            self.epistemic_disposition_profiles.get(&agent).cloned()
        }

        fn theft_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<worldwake_core::TheftDispositionProfile> {
            self.theft_disposition_profiles.get(&agent).cloned()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
        fn justice_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<worldwake_core::JusticeDispositionProfile> {
            self.justice_disposition_profiles.get(&agent).cloned()
        }
        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
        }

        fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
            self.told_beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn told_belief_memory(
            &self,
            actor: EntityId,
            counterparty: EntityId,
            topic: &TellTopic,
        ) -> Option<ToldBeliefMemory> {
            let profile = self.tell_profile(actor)?;
            self.told_beliefs
                .get(&actor)
                .and_then(|memories| {
                    memories
                        .iter()
                        .find(|(key, _)| {
                            *key == TellMemoryKey {
                                counterparty,
                                topic: *topic,
                            }
                        })
                        .map(|(_, memory)| memory)
                })
                .filter(|memory| {
                    self.current_tick.0.saturating_sub(memory.told_tick.0)
                        <= profile.conversation_memory_retention_ticks
                })
                .cloned()
        }

        fn recipient_knowledge_status(
            &self,
            actor: EntityId,
            counterparty: EntityId,
            topic: &TellTopic,
        ) -> Option<RecipientKnowledgeStatus> {
            let current_state = match topic {
                TellTopic::EntityBelief { subject } => {
                    SharedTellState::EntityBelief(worldwake_core::to_shared_belief_snapshot(
                        self.beliefs
                            .get(&actor)?
                            .iter()
                            .find(|(known_subject, _)| *known_subject == *subject)
                            .map(|(_, belief)| belief)?,
                    ))
                }
                TellTopic::SocialObservation { observation } => {
                    SharedTellState::SocialObservation(*observation)
                }
                TellTopic::InstitutionalClaim { claim } => SharedTellState::InstitutionalClaim(
                    self.institutional_claims
                        .values()
                        .flat_map(|beliefs| beliefs.iter())
                        .filter(|belief| belief.claim == *claim)
                        .max_by_key(|belief| {
                            (
                                std::cmp::Reverse(
                                    worldwake_core::institutional_knowledge_chain_len(
                                        belief.source,
                                    ),
                                ),
                                belief.learned_tick,
                                belief.learned_at,
                            )
                        })
                        .map(|belief| worldwake_core::SharedInstitutionalBelief {
                            claim: belief.claim,
                            source: belief.source,
                        })?,
                ),
            };
            let remembered = self.told_belief_memory(actor, counterparty, topic);
            let had_raw_memory = self.told_beliefs.get(&actor).is_some_and(|memories| {
                memories.iter().any(|(key, _)| {
                    *key == TellMemoryKey {
                        counterparty,
                        topic: *topic,
                    }
                })
            });
            self.tell_profile(actor)?;

            Some(match remembered.as_ref() {
                Some(memory) => {
                    worldwake_core::recipient_knowledge_status(&current_state, Some(memory))
                }
                None if had_raw_memory => {
                    RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
                }
                None => RecipientKnowledgeStatus::UnknownToSpeaker,
            })
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }

        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }

        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.sellers
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }

        fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
            self.known_recipes.get(&agent).cloned().unwrap_or_default()
        }

        fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
            self.workstations
                .get(&(place, tag))
                .cloned()
                .unwrap_or_default()
        }

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.sources_at
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }

        fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.corpses_at.get(&place).cloned().unwrap_or_default()
        }

        fn record_data(&self, record: EntityId) -> Option<RecordData> {
            self.record_data.get(&record).cloned()
        }

        fn office_data(&self, office: EntityId) -> Option<OfficeData> {
            self.office_data.get(&office).cloned()
        }

        fn believed_office_holder(
            &self,
            office: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.office_holder_beliefs
                .get(&office)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn believed_force_controller(
            &self,
            office: EntityId,
        ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
            self.force_controller_beliefs
                .get(&office)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn believed_membership(
            &self,
            faction: EntityId,
            member: EntityId,
        ) -> InstitutionalBeliefRead<bool> {
            if self
                .factions_by_member
                .get(&member)
                .is_some_and(|factions| factions.contains(&faction))
            {
                InstitutionalBeliefRead::Certain(true)
            } else {
                InstitutionalBeliefRead::Unknown
            }
        }

        fn believed_faction_rally_point(
            &self,
            faction: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.faction_rally_point_beliefs
                .get(&faction)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
            self.loyalties.get(&(subject, target)).copied()
        }

        fn believed_support_declaration(
            &self,
            office: EntityId,
            supporter: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.support_declaration_beliefs
                .get(&(office, supporter))
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent_places(place)
                .into_iter()
                .map(|adjacent| (adjacent, NonZeroU32::new(1).unwrap()))
                .collect()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }

        fn institutional_belief_claims(
            &self,
            agent: EntityId,
            key: InstitutionalBeliefKey,
        ) -> Vec<BelievedInstitutionalClaim> {
            self.institutional_claims
                .get(&(agent, key))
                .cloned()
                .unwrap_or_default()
        }

        fn violation_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<worldwake_core::ViolationDispositionProfile> {
            self.violation_disposition_profiles.get(&agent).cloned()
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn patrol_profile(base: u16) -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks: 5,
            dwell_vigilance_scale_ticks: 5,
            vigilance: pm(500),
            route_adaptation_sensitivity: pm(400),
            patrol_motive_weight: pm(base),
        }
    }

    fn hunger(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(value), pm(0), pm(0), pm(0), pm(0))
    }

    fn thirst(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(0), pm(value), pm(0), pm(0), pm(0))
    }

    fn fatigue(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(0), pm(0), pm(value), pm(0), pm(0))
    }

    fn dirtiness(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(value))
    }

    fn wound(severity: u16) -> Wound {
        Wound {
            id: WoundId(u64::from(severity)),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
            severity: pm(severity),
            inflicted_at: Tick(1),
            bleed_rate_per_tick: pm(0),
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

    fn sample_recipe(
        outputs: Vec<(CommodityKind, Quantity)>,
        inputs: Vec<(CommodityKind, Quantity)>,
        tag: WorkstationTag,
    ) -> RecipeDefinition {
        RecipeDefinition {
            name: "sample".to_string(),
            inputs,
            outputs,
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(tag),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        }
    }

    fn contains_goal(candidates: &[crate::GroundedGoal], goal: GoalKind) -> bool {
        candidates
            .iter()
            .any(|candidate| candidate.key.kind == goal)
    }

    #[test]
    fn patrol_candidates_emit_next_waypoint_when_route_and_profile_exist() {
        let agent = entity(1);
        let square = entity(10);
        let gate = entity(11);
        let hall = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, square);
        view.entities_at.insert(square, vec![agent]);
        view.patrol_profiles.insert(agent, patrol_profile(550));
        view.patrol_routes.insert(
            agent,
            PatrolRoute {
                assigned_places: vec![square, gate, hall],
                current_index: 1,
            },
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(result.candidates.iter().any(|candidate| {
            candidate.key.kind == GoalKind::Patrol { place: gate }
                && candidate.anchor == OpportunityAnchor::Place(gate)
                && candidate.evidence_places == BTreeSet::from([gate])
        }));
    }

    #[test]
    fn patrol_candidates_require_complete_patrol_state_and_valid_index() {
        let agent = entity(1);
        let square = entity(10);
        let gate = entity(11);
        let mut missing_profile = TestBeliefView::default();
        missing_profile.alive.insert(agent);
        missing_profile.entity_kinds.insert(agent, EntityKind::Agent);
        missing_profile.effective_places.insert(agent, square);
        missing_profile.entities_at.insert(square, vec![agent]);
        missing_profile.patrol_routes.insert(
            agent,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            },
        );

        let missing_profile_result = generate_candidates_with_travel_horizon(
            &missing_profile,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );
        assert!(!contains_goal(
            &missing_profile_result.candidates,
            GoalKind::Patrol { place: square }
        ));

        let mut invalid_route = missing_profile;
        invalid_route.patrol_profiles.insert(agent, patrol_profile(550));
        invalid_route.patrol_routes.insert(
            agent,
            PatrolRoute {
                assigned_places: vec![square],
                current_index: 3,
            },
        );

        let invalid_route_result = generate_candidates_with_travel_horizon(
            &invalid_route,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );
        assert!(
            !invalid_route_result
                .candidates
                .iter()
                .any(|candidate| matches!(candidate.key.kind, GoalKind::Patrol { .. }))
        );
    }

    fn goals_for<'a>(
        candidates: &'a [crate::GroundedGoal],
        goal: &GoalKind,
    ) -> Vec<&'a crate::GroundedGoal> {
        candidates
            .iter()
            .filter(|candidate| candidate.key.kind == *goal)
            .collect()
    }

    fn contains_political_omission(
        diagnostics: &CandidateGenerationDiagnostics,
        family: PoliticalGoalFamily,
        office: EntityId,
        candidate: Option<EntityId>,
        reason: PoliticalCandidateOmissionReason,
    ) -> bool {
        diagnostics.omitted_political.iter().any(|omission| {
            omission.family == family
                && omission.office == office
                && omission.candidate == candidate
                && omission.reason == reason
        })
    }

    fn contains_bandit_omission(
        diagnostics: &CandidateGenerationDiagnostics,
        family: BanditGoalFamily,
        faction: EntityId,
        reason: BanditCandidateOmissionReason,
    ) -> bool {
        diagnostics.omitted_bandit.iter().any(|omission| {
            *omission
                == BanditCandidateOmission {
                    family,
                    faction,
                    reason,
                }
        })
    }

    fn contains_social_omission(
        diagnostics: &CandidateGenerationDiagnostics,
        listener: EntityId,
        subject: EntityId,
        reason: TellTopicOmissionReason,
    ) -> bool {
        diagnostics.omitted_social.iter().any(|omission| {
            *omission
                == SocialCandidateOmission {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                    reason,
                }
        })
    }

    fn evidence_trace_for_goal(
        diagnostics: &CandidateGenerationDiagnostics,
        goal: GoalKey,
    ) -> &CandidateEvidenceTrace {
        diagnostics
            .evidence
            .values()
            .find(|trace| trace.opportunity.goal_key == goal)
            .expect("goal should have evidence trace")
    }

    fn believed_state(observed_tick: u64, source: PerceptionSource) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: None,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            observed_tick: Tick(observed_tick),
            source,
        }
    }

    fn known_entity(subject: EntityId, place: EntityId) -> (EntityId, BelievedEntityState) {
        (
            subject,
            BelievedEntityState {
                last_known_place: Some(place),
                ..believed_state(5, PerceptionSource::DirectObservation)
            },
        )
    }

    fn told_memory(
        counterparty: EntityId,
        subject: EntityId,
        told_tick: u64,
        belief: &BelievedEntityState,
    ) -> (TellMemoryKey, ToldBeliefMemory) {
        (
            TellMemoryKey {
                counterparty,
                topic: TellTopic::EntityBelief { subject },
            },
            ToldBeliefMemory {
                shared_state: SharedTellState::EntityBelief(
                    worldwake_core::to_shared_belief_snapshot(belief),
                ),
                told_tick: Tick(told_tick),
            },
        )
    }

    fn vacant_office(title: &str, jurisdiction: EntityId, faction: EntityId) -> OfficeData {
        OfficeData {
            title: title.to_string(),
            jurisdiction,
            succession_law: worldwake_core::SuccessionLaw::Support,
            eligibility_rules: vec![EligibilityRule::FactionMember(faction)],
            succession_period_ticks: 8,
            vacancy_since: Some(Tick(3)),
        }
    }

    fn office_register_record(
        issuer: EntityId,
        home_place: EntityId,
        office: EntityId,
    ) -> RecordData {
        RecordData {
            record_kind: RecordKind::OfficeRegister,
            home_place,
            issuer,
            consultation_ticks: 4,
            max_entries_per_consult: 2,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id: RecordEntryId(0),
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(3),
                },
                recorded_tick: Tick(3),
                supersedes: None,
            }],
            next_entry_id: 1,
        }
    }

    fn crime_register_record(
        issuer: EntityId,
        home_place: EntityId,
        entry_id: RecordEntryId,
        claim: InstitutionalClaim,
    ) -> RecordData {
        RecordData {
            record_kind: RecordKind::CrimeRegister,
            home_place,
            issuer,
            consultation_ticks: 1,
            max_entries_per_consult: 8,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id,
                claim,
                recorded_tick: Tick(3),
                supersedes: None,
            }],
            next_entry_id: entry_id.0 + 1,
        }
    }

    fn default_justice_profile() -> worldwake_core::JusticeDispositionProfile {
        worldwake_core::JusticeDispositionProfile {
            accusation_motive_weight: pm(700),
            fine_severity: pm(500),
        }
    }

    fn default_epistemic_profile() -> EpistemicDispositionProfile {
        EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: pm(500),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
        }
    }

    #[test]
    fn dead_agent_generates_zero_candidates() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.dead.insert(agent);
        let recipes = RecipeRegistry::new();

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(candidates.is_empty());
    }

    #[test]
    fn owned_food_emits_consume_goal_when_hungry() {
        let agent = entity(1);
        let place = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(bread, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.direct_possessions.insert(agent, vec![bread]);
        view.direct_possessors.insert(bread, agent);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.controllable.insert((agent, bread));
        view.controlled_entities.insert(agent);
        view.commodity_quantities
            .insert((agent, CommodityKind::Bread), Quantity(1));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }
        ));
    }

    #[test]
    fn merchant_does_not_emit_consume_owned_for_sale_commodity() {
        let agent = entity(1);
        let place = entity(10);
        let apple = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(apple, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(apple, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.direct_possessions.insert(agent, vec![apple]);
        view.direct_possessors.insert(apple, agent);
        view.lot_commodities.insert(apple, CommodityKind::Apple);
        view.consumable_profiles.insert(
            apple,
            CommodityKind::Apple.spec().consumable_profile.unwrap(),
        );
        view.controllable.insert((agent, apple));
        view.controlled_entities.insert(agent);
        view.commodity_quantities
            .insert((agent, CommodityKind::Apple), Quantity(1));
        // Mark Apple as sale stock for this merchant.
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: std::iter::once(CommodityKind::Apple).collect(),
                home_market: Some(place),
            },
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        // Must NOT emit ConsumeOwnedCommodity for a sale commodity.
        assert!(!contains_goal(
            &candidates,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Apple,
            }
        ));
    }

    #[test]
    fn owned_water_emits_consume_goal_when_thirsty() {
        let agent = entity(1);
        let place = entity(10);
        let water = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(water, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(water, place);
        view.homeostatic_needs.insert(agent, thirst(200));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.direct_possessions.insert(agent, vec![water]);
        view.direct_possessors.insert(water, agent);
        view.lot_commodities.insert(water, CommodityKind::Water);
        view.consumable_profiles.insert(
            water,
            CommodityKind::Water.spec().consumable_profile.unwrap(),
        );
        view.controllable.insert((agent, water));
        view.controlled_entities.insert(agent);
        view.commodity_quantities
            .insert((agent, CommodityKind::Water), Quantity(1));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water,
            }
        ));
    }

    #[test]
    fn local_seller_emits_acquire_goal_when_hungry_and_no_food_owned() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn low_confidence_evidence_keeps_originating_goal_without_standalone_epistemic_goal() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        view.beliefs
            .insert(agent, vec![known_entity(seller, place)]);
        view.epistemic_disposition_profiles
            .insert(agent, default_epistemic_profile());

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(50),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert_eq!(
            candidates.len(),
            1,
            "low-confidence prerequisite evidence should keep the originating acquisition opportunity only"
        );
    }

    #[test]
    fn stale_resource_source_stays_on_restock_goal_without_standalone_epistemic_goal() {
        let agent = entity(1);
        let camp = entity(10);
        let crossroads = entity(11);
        let orchard = entity(12);
        let workstation = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(camp, EntityKind::Place);
        view.entity_kinds.insert(crossroads, EntityKind::Place);
        view.entity_kinds.insert(orchard, EntityKind::Place);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, camp);
        view.effective_places.insert(workstation, orchard);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(camp, vec![crossroads]);
        view.adjacent_places.insert(crossroads, vec![camp, orchard]);
        view.adjacent_places.insert(orchard, vec![crossroads]);
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((orchard, WorkstationTag::OrchardRow), vec![workstation]);
        let source = ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        };
        view.resource_sources.insert(workstation, source.clone());
        view.beliefs.insert(
            agent,
            vec![(
                workstation,
                BelievedEntityState {
                    last_known_place: Some(orchard),
                    resource_source: Some(source),
                    ..believed_state(0, PerceptionSource::DirectObservation)
                },
            )],
        );
        view.epistemic_disposition_profiles
            .insert(agent, default_epistemic_profile());

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Apple, Quantity(2))],
            Vec::new(),
            WorkstationTag::OrchardRow,
        ));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &recipes,
            Tick(50),
            2,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0),
            }
        ));
        assert_eq!(
            result.candidates.len(),
            1,
            "stale source evidence should remain on the originating production opportunity without emitting a second verification-only candidate"
        );
    }

    #[test]
    fn remote_harvest_source_within_travel_horizon_emits_produce_goal() {
        let agent = entity(1);
        let camp = entity(10);
        let crossroads = entity(11);
        let orchard = entity(12);
        let workstation = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(camp, EntityKind::Place);
        view.entity_kinds.insert(crossroads, EntityKind::Place);
        view.entity_kinds.insert(orchard, EntityKind::Place);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, camp);
        view.effective_places.insert(workstation, orchard);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(camp, vec![crossroads]);
        view.adjacent_places.insert(crossroads, vec![camp, orchard]);
        view.adjacent_places.insert(orchard, vec![crossroads]);
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((orchard, WorkstationTag::OrchardRow), vec![workstation]);
        view.resource_sources.insert(
            workstation,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(10),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Apple, Quantity(2))],
            Vec::new(),
            WorkstationTag::OrchardRow,
        ));

        let candidates = super::generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &recipes,
            Tick(5),
            2,
            false,
        );
        let goal = candidates
            .candidates
            .iter()
            .find(|candidate| {
                candidate.key.kind
                    == GoalKind::ProduceCommodity {
                        recipe_id: RecipeId(0),
                    }
            })
            .expect("reachable remote harvest source should emit produce goal");

        assert_eq!(
            goal.anchor,
            worldwake_core::OpportunityAnchor::Place(orchard)
        );
        assert!(goal.evidence_entities.contains(&workstation));
        assert!(!contains_goal(
            &candidates.candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn hunger_below_low_band_emits_no_hunger_goals() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.homeostatic_needs.insert(agent, hunger(50));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!candidates.iter().any(|candidate| {
            matches!(
                candidate.key.kind,
                GoalKind::ConsumeOwnedCommodity { .. }
                    | GoalKind::AcquireCommodity {
                        purpose: CommodityPurpose::SelfConsume,
                        ..
                    }
            )
        }));
    }

    #[test]
    fn acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence() {
        let agent = entity(1);
        let home = entity(10);
        let orchard = entity(11);
        let market = entity(12);
        let seller = entity(2);
        let bread_lot = entity(3);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller, bread_lot]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(home, EntityKind::Place);
        view.entity_kinds.insert(orchard, EntityKind::Place);
        view.entity_kinds.insert(market, EntityKind::Place);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.entity_kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(seller, orchard);
        view.effective_places.insert(bread_lot, market);
        view.entities_at.insert(market, vec![bread_lot]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(home, vec![orchard, market]);
        view.adjacent_places.insert(orchard, vec![home]);
        view.adjacent_places.insert(market, vec![home]);
        view.sellers
            .insert((orchard, CommodityKind::Bread), vec![seller]);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );
        let candidates = result.candidates;

        let acquire_goals = goals_for(&candidates, &goal);
        assert_eq!(acquire_goals.len(), 2);
        assert_eq!(result.diagnostics.evidence.len(), 2);

        let orchard_goal = acquire_goals
            .iter()
            .find(|candidate| candidate.anchor == worldwake_core::OpportunityAnchor::Place(orchard))
            .expect("orchard opportunity should be emitted");
        assert_eq!(orchard_goal.evidence_places, BTreeSet::from([orchard]));
        assert_eq!(orchard_goal.evidence_entities, BTreeSet::from([seller]));

        let market_goal = acquire_goals
            .iter()
            .find(|candidate| candidate.anchor == worldwake_core::OpportunityAnchor::Place(market))
            .expect("market opportunity should be emitted");
        assert_eq!(market_goal.evidence_places, BTreeSet::from([market]));
        assert_eq!(market_goal.evidence_entities, BTreeSet::from([bread_lot]));

        let orchard_trace = result
            .diagnostics
            .evidence
            .get(&worldwake_core::OpportunityKey {
                goal_key: orchard_goal.key,
                anchor: orchard_goal.anchor,
            })
            .expect("orchard opportunity should keep a distinct evidence trace");
        assert_eq!(
            orchard_trace.opportunity.anchor,
            worldwake_core::OpportunityAnchor::Place(orchard)
        );
        assert_eq!(orchard_trace.contributors.len(), 1);
        assert_eq!(orchard_trace.contributors[0].entity, seller);

        let market_trace = result
            .diagnostics
            .evidence
            .get(&worldwake_core::OpportunityKey {
                goal_key: market_goal.key,
                anchor: market_goal.anchor,
            })
            .expect("market opportunity should keep a distinct evidence trace");
        assert_eq!(
            market_trace.opportunity.anchor,
            worldwake_core::OpportunityAnchor::Place(market)
        );
        assert_eq!(market_trace.contributors.len(), 1);
        assert_eq!(market_trace.contributors[0].entity, bread_lot);
    }

    #[test]
    fn blocked_acquire_place_only_suppresses_matching_opportunity() {
        let agent = entity(1);
        let home = entity(10);
        let orchard = entity(11);
        let market = entity(12);
        let orchard_seller = entity(2);
        let market_seller = entity(3);
        let key = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, orchard_seller, market_seller]);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(orchard_seller, orchard);
        view.effective_places.insert(market_seller, market);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(home, vec![orchard, market]);
        view.adjacent_places.insert(orchard, vec![home]);
        view.adjacent_places.insert(market, vec![home]);
        view.sellers
            .insert((orchard, CommodityKind::Bread), vec![orchard_seller]);
        view.sellers
            .insert((market, CommodityKind::Bread), vec![market_seller]);

        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: key,
                place: Some(orchard),
                target: None,
                action_def: None,
            },
            blocking_fact: BlockingFact::NoKnownSeller,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
        });

        let candidates =
            generate_candidates(&view, agent, &blocked, &RecipeRegistry::new(), Tick(5));

        let acquire_goals = goals_for(
            &candidates,
            &GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            },
        );
        assert_eq!(acquire_goals.len(), 1);
        assert_eq!(
            acquire_goals[0].anchor,
            worldwake_core::OpportunityAnchor::Place(market)
        );
    }

    #[test]
    fn diagnostics_record_desire_fully_blocked_when_all_opportunities_are_filtered() {
        let agent = entity(1);
        let home = entity(10);
        let orchard = entity(11);
        let market = entity(12);
        let orchard_seller = entity(2);
        let market_seller = entity(3);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let key = GoalKey::from(goal);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, orchard_seller, market_seller]);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(orchard_seller, orchard);
        view.effective_places.insert(market_seller, market);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(home, vec![orchard, market]);
        view.adjacent_places.insert(orchard, vec![home]);
        view.adjacent_places.insert(market, vec![home]);
        view.sellers
            .insert((orchard, CommodityKind::Bread), vec![orchard_seller]);
        view.sellers
            .insert((market, CommodityKind::Bread), vec![market_seller]);

        let mut blocked = BlockedIntentMemory::default();
        for place in [orchard, market] {
            blocked.record(BlockedIntent {
                blocker_key: BlockerKey {
                    goal_key: key,
                    place: Some(place),
                    target: None,
                    action_def: None,
                },
                blocking_fact: BlockingFact::NoKnownSeller,
                diagnostic_context: None,
                observed_tick: Tick(1),
                expires_tick: Tick(10),
            });
        }

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            goals_for(&result.candidates, &goal).is_empty(),
            "all acquire opportunities should be filtered out"
        );
        assert_eq!(result.diagnostics.fully_blocked_desires.len(), 1);
        let diagnostic = &result.diagnostics.fully_blocked_desires[0];
        assert_eq!(diagnostic.goal_key, key);
        assert_eq!(
            diagnostic.blocked_opportunities,
            vec![
                worldwake_core::OpportunityKey {
                    goal_key: key,
                    anchor: worldwake_core::OpportunityAnchor::Place(orchard),
                },
                worldwake_core::OpportunityKey {
                    goal_key: key,
                    anchor: worldwake_core::OpportunityAnchor::Place(market),
                },
            ]
        );
    }

    #[test]
    fn diagnostics_omit_desire_fully_blocked_when_one_opportunity_survives() {
        let agent = entity(1);
        let home = entity(10);
        let orchard = entity(11);
        let market = entity(12);
        let orchard_seller = entity(2);
        let market_seller = entity(3);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let key = GoalKey::from(goal);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, orchard_seller, market_seller]);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(orchard_seller, orchard);
        view.effective_places.insert(market_seller, market);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.adjacent_places.insert(home, vec![orchard, market]);
        view.adjacent_places.insert(orchard, vec![home]);
        view.adjacent_places.insert(market, vec![home]);
        view.sellers
            .insert((orchard, CommodityKind::Bread), vec![orchard_seller]);
        view.sellers
            .insert((market, CommodityKind::Bread), vec![market_seller]);

        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: key,
                place: Some(orchard),
                target: None,
                action_def: None,
            },
            blocking_fact: BlockingFact::NoKnownSeller,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
        });

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert_eq!(goals_for(&result.candidates, &goal).len(), 1);
        assert!(
            result.diagnostics.fully_blocked_desires.is_empty(),
            "a surviving sibling opportunity must suppress the desire-level diagnostic"
        );
    }

    #[test]
    fn hunger_emits_acquire_goal_for_local_unpossessed_food_lot() {
        let agent = entity(1);
        let place = entity(10);
        let bread_lot = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, bread_lot]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(bread_lot, place);
        view.entities_at.insert(place, vec![agent, bread_lot]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread_lot,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn local_unpossessed_food_relief_suppresses_duplicate_produce_goal() {
        let agent = entity(1);
        let place = entity(10);
        let apple_lot = entity(11);
        let workstation = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, apple_lot, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(apple_lot, EntityKind::ItemLot);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(apple_lot, place);
        view.effective_places.insert(workstation, place);
        view.entities_at
            .insert(place, vec![agent, apple_lot, workstation]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.lot_commodities.insert(apple_lot, CommodityKind::Apple);
        view.consumable_profiles.insert(
            apple_lot,
            CommodityKind::Apple.spec().consumable_profile.unwrap(),
        );
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::OrchardRow), vec![workstation]);
        view.resource_sources.insert(
            workstation,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(10),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Apple, Quantity(2))],
            Vec::new(),
            WorkstationTag::OrchardRow,
        ));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0)
            }
        ));
    }

    #[test]
    fn fatigue_and_bladder_emit_sleep_and_relieve() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.homeostatic_needs.insert(
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), pm(350), pm(400), pm(0)),
        );
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(&candidates, GoalKind::Sleep));
        assert!(contains_goal(&candidates, GoalKind::Relieve));
    }

    #[test]
    fn wash_requires_dirtiness_and_local_water() {
        let agent = entity(1);
        let place = entity(10);
        let water = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.homeostatic_needs.insert(agent, dirtiness(450));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.entity_kinds.insert(water, EntityKind::ItemLot);
        view.effective_places.insert(water, place);
        view.direct_possessions.insert(agent, vec![water]);
        view.direct_possessors.insert(water, agent);
        view.lot_commodities.insert(water, CommodityKind::Water);
        view.controllable.insert((agent, water));
        view.commodity_quantities
            .insert((agent, CommodityKind::Water), Quantity(1));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(&candidates, GoalKind::Wash));

        let mut no_water_view = view;
        no_water_view.direct_possessions.clear();
        no_water_view.controllable.clear();
        no_water_view.commodity_quantities.clear();
        let no_water_candidates = generate_candidates(
            &no_water_view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(!contains_goal(&no_water_candidates, GoalKind::Wash));
    }

    #[test]
    fn reduce_danger_requires_pressure_and_mitigation_path() {
        let agent = entity(1);
        let place = entity(10);
        let adjacent = entity(11);
        let attacker = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, attacker]);
        view.effective_places.insert(agent, place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![attacker]);

        let none = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(!contains_goal(&none, GoalKind::ReduceDanger));

        view.hostiles.clear();
        view.attackers.insert(agent, vec![attacker]);
        view.adjacent_places.insert(place, vec![adjacent]);
        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(contains_goal(&candidates, GoalKind::ReduceDanger));
    }

    #[test]
    fn reduce_danger_is_not_emitted_for_medium_visible_hostility() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(10);
        let adjacent = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, hostile]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(hostile, place);
        view.entities_at.insert(place, vec![agent, hostile]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![hostile]);
        view.adjacent_places.insert(place, vec![adjacent]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(&candidates, GoalKind::ReduceDanger));
    }

    #[test]
    fn engage_hostile_emits_for_local_visible_hostile_that_is_not_attacking() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, hostile]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(hostile, place);
        view.entities_at.insert(place, vec![agent, hostile]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![hostile]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::EngageHostile { target: hostile }
        ));
    }

    #[test]
    fn bandit_with_local_non_faction_agent_emits_raid_target_instead_of_engage_hostile() {
        let agent = entity(1);
        let traveler = entity(2);
        let faction = entity(30);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, traveler]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(traveler, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(traveler, place);
        view.entities_at.insert(place, vec![agent, traveler]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![traveler]);
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::RaidTarget { target: traveler }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::EngageHostile { target: traveler }
        ));
    }

    #[test]
    fn bandit_does_not_raid_same_faction_member() {
        let agent = entity(1);
        let ally = entity(2);
        let faction = entity(30);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, ally]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(ally, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(ally, place);
        view.entities_at.insert(place, vec![agent, ally]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![ally]);
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(ally, vec![faction]);
        view.bandit_factions.insert(faction);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::RaidTarget { target: ally }
        ));
    }

    #[test]
    fn bandit_raid_target_is_suppressed_by_wound_deterrence() {
        let agent = entity(1);
        let traveler = entity(2);
        let faction = entity(30);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, traveler]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(traveler, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(traveler, place);
        view.entities_at.insert(place, vec![agent, traveler]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.bandit_flee_thresholds.insert(faction, pm(300));
        view.courage.insert(agent, pm(200));
        view.wounds.insert(agent, vec![wound(120), wound(120)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::RaidTarget { target: traveler }
        ));

        view.wounds.insert(agent, vec![wound(120), wound(100)]);
        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::RaidTarget { target: traveler }
        ));
    }

    #[test]
    fn blocked_raid_target_is_filtered_by_blocked_memory() {
        let agent = entity(1);
        let traveler = entity(2);
        let faction = entity(30);
        let place = entity(10);
        let goal = GoalKind::RaidTarget { target: traveler };
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, traveler]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(traveler, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(traveler, place);
        view.entities_at.insert(place, vec![agent, traveler]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![traveler]);
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);

        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: GoalKey::from(goal),
                place: Some(place),
                target: Some(traveler),
                action_def: None,
            },
            blocking_fact: BlockingFact::CombatTooRisky,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(10),
        });

        let candidates =
            generate_candidates(&view, agent, &blocked, &RecipeRegistry::new(), Tick(5));

        assert!(!contains_goal(&candidates, goal));
    }

    #[test]
    fn regroup_with_faction_requires_rally_point_belief() {
        let agent = entity(1);
        let faction = entity(30);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::RegroupWithFaction { faction }
        ));
        assert!(contains_bandit_omission(
            &result.diagnostics,
            BanditGoalFamily::RegroupWithFaction,
            faction,
            BanditCandidateOmissionReason::MissingRallyBelief
        ));
    }

    #[test]
    fn regroup_with_faction_emits_when_rally_point_belief_exists() {
        let agent = entity(1);
        let faction = entity(30);
        let place = entity(10);
        let rally_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.faction_rally_point_beliefs
            .insert(faction, InstitutionalBeliefRead::Certain(Some(rally_place)));
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::FactionRallyPointOf { faction },
            ),
            vec![BelievedInstitutionalClaim {
                claim: InstitutionalClaim::FactionRallyPoint {
                    faction,
                    rally_place: Some(rally_place),
                    effective_tick: Tick(4),
                },
                source: InstitutionalKnowledgeSource::DirectObservation,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::RegroupWithFaction { faction }
        ));
        let trace = evidence_trace_for_goal(
            &result.diagnostics,
            GoalKey::from(GoalKind::RegroupWithFaction { faction }),
        );
        assert!(
            trace
                .knowledge_path
                .institutional_beliefs
                .contains(&InstitutionalBeliefProvenance {
                    claim: InstitutionalClaim::FactionRallyPoint {
                        faction,
                        rally_place: Some(rally_place),
                        effective_tick: Tick(4),
                    },
                    source: InstitutionalKnowledgeSource::DirectObservation,
                    learned_tick: Tick(4),
                    learned_at: Some(place),
                }),
            "regroup knowledge path should record rally doctrine provenance, got {:?}",
            trace.knowledge_path.institutional_beliefs,
        );
    }

    #[test]
    fn regroup_with_faction_is_suppressed_while_agent_stands_in_active_faction_camp() {
        let agent = entity(1);
        let faction = entity(30);
        let place = entity(10);
        let rally_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.local_bandit_camps.insert(place, faction);
        view.faction_rally_point_beliefs
            .insert(faction, InstitutionalBeliefRead::Certain(Some(rally_place)));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::RegroupWithFaction { faction }
        ));
        assert!(contains_bandit_omission(
            &result.diagnostics,
            BanditGoalFamily::RegroupWithFaction,
            faction,
            BanditCandidateOmissionReason::AlreadySafeInObservedActiveCamp
        ));
    }

    #[test]
    fn establish_bandit_camp_emits_at_rally_when_local_edible_supplies_are_controlled() {
        let agent = entity(1);
        let faction = entity(30);
        let rally_place = entity(11);
        let bread = entity(40);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(rally_place, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, rally_place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.faction_rally_point_beliefs
            .insert(faction, InstitutionalBeliefRead::Certain(Some(rally_place)));
        view.direct_possessions.insert(agent, vec![bread]);
        view.direct_possessors.insert(bread, agent);
        view.controllable.insert((agent, bread));
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread
                .spec()
                .consumable_profile
                .expect("bread should stay edible"),
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::EstablishBanditCamp { faction }
        ));
    }

    #[test]
    fn establish_bandit_camp_requires_local_controlled_edible_supplies() {
        let agent = entity(1);
        let faction = entity(30);
        let rally_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(rally_place, EntityKind::Place);
        view.effective_places.insert(agent, rally_place);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.faction_rally_point_beliefs
            .insert(faction, InstitutionalBeliefRead::Certain(Some(rally_place)));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::EstablishBanditCamp { faction }
        ));
        assert!(contains_bandit_omission(
            &result.diagnostics,
            BanditGoalFamily::EstablishBanditCamp,
            faction,
            BanditCandidateOmissionReason::MissingLocalControlledEdibleSupplies
        ));
    }

    #[test]
    fn engage_hostile_is_suppressed_for_current_attackers() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, hostile]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(hostile, place);
        view.entities_at.insert(place, vec![agent, hostile]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![hostile]);
        view.attackers.insert(agent, vec![hostile]);
        view.adjacent_places.insert(place, vec![entity(11)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::EngageHostile { target: hostile }
        ));
        assert!(contains_goal(&candidates, GoalKind::ReduceDanger));
    }

    #[test]
    fn engage_hostile_is_suppressed_when_high_danger_requires_defense() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(10);
        let refuge = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, hostile, refuge]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.entity_kinds.insert(refuge, EntityKind::Place);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(hostile, place);
        view.entities_at.insert(place, vec![agent, hostile]);
        view.adjacent_places.insert(place, vec![refuge]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![hostile]);
        view.wounds.insert(agent, vec![wound(120)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(0),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::EngageHostile { target: hostile }
        ));
        assert!(contains_goal(&candidates, GoalKind::ReduceDanger));
    }

    #[test]
    fn self_wounded_emits_treat_wounds_without_medicine() {
        let agent = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.entities_at.insert(place, vec![agent]);
        view.wounds.insert(agent, vec![wound(100)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::TreatWounds { patient: agent }
        ));
    }

    #[test]
    fn self_wounded_emits_treat_wounds_with_medicine() {
        let agent = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.entities_at.insert(place, vec![agent]);
        view.wounds.insert(agent, vec![wound(100)]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Medicine), Quantity(1));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::TreatWounds { patient: agent }
        ));
    }

    #[test]
    fn directly_observed_wounded_other_emits_treat_wounds() {
        let agent = entity(1);
        let patient = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(patient, place);
        view.entities_at.insert(place, vec![agent, patient]);
        view.beliefs.insert(
            agent,
            vec![(
                patient,
                BelievedEntityState {
                    wounds: vec![wound(100)],
                    ..believed_state(5, PerceptionSource::DirectObservation)
                },
            )],
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::TreatWounds { patient }
        ));
    }

    #[test]
    fn report_source_wounded_other_does_not_emit_care_goal() {
        let agent = entity(1);
        let patient = entity(2);
        let reporter = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.beliefs.insert(
            agent,
            vec![(
                patient,
                BelievedEntityState {
                    wounds: vec![wound(100)],
                    ..believed_state(
                        5,
                        PerceptionSource::Report {
                            from: reporter,
                            chain_len: 1,
                        },
                    )
                },
            )],
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::TreatWounds { patient }
        ));
    }

    #[test]
    fn rumor_source_wounded_other_does_not_emit_care_goal() {
        let agent = entity(1);
        let patient = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.beliefs.insert(
            agent,
            vec![(
                patient,
                BelievedEntityState {
                    wounds: vec![wound(100)],
                    ..believed_state(5, PerceptionSource::Rumor { chain_len: 2 })
                },
            )],
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::TreatWounds { patient }
        ));
    }

    #[test]
    fn satisfiable_recipe_with_current_need_emits_produce_goal() {
        let agent = entity(1);
        let place = entity(10);
        let workstation = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::Mill), vec![workstation]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));
        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Bread, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(2))],
            WorkstationTag::Mill,
        ));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0)
            }
        ));
    }

    #[test]
    fn missing_recipe_input_emits_produce_goal_without_recipe_input_proxy() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let workstation = entity(20);
        let recipe_id = RecipeId(0);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.effective_places.insert(workstation, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![recipe_id]);
        view.workstations
            .insert((place, WorkstationTag::Mill), vec![workstation]);
        view.sellers
            .insert((place, CommodityKind::Firewood), vec![seller]);

        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity { recipe_id }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            }
        ));
    }

    #[test]
    fn reachable_remote_workstation_keeps_missing_input_produce_goal_emittable() {
        let agent = entity(1);
        let seller = entity(2);
        let origin = entity(10);
        let remote = entity(11);
        let workstation = entity(20);
        let recipe_id = RecipeId(0);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(seller, remote);
        view.effective_places.insert(workstation, remote);
        view.adjacent_places.insert(origin, vec![remote]);
        view.adjacent_places.insert(remote, vec![origin]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![recipe_id]);
        view.workstations
            .insert((remote, WorkstationTag::Mill), vec![workstation]);
        view.sellers
            .insert((remote, CommodityKind::Firewood), vec![seller]);

        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity { recipe_id }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn depleted_resource_sources_are_excluded_from_produce_goal_evidence() {
        let agent = entity(1);
        let origin = entity(10);
        let orchard = entity(11);
        let bandit_camp = entity(12);
        let mill = entity(20);
        let depleted_source = entity(21);
        let stocked_source = entity(22);
        let recipe_id = RecipeId(0);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([agent, mill, depleted_source, stocked_source]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(mill, EntityKind::Facility);
        view.entity_kinds
            .insert(depleted_source, EntityKind::Facility);
        view.entity_kinds
            .insert(stocked_source, EntityKind::Facility);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(mill, origin);
        view.effective_places.insert(depleted_source, orchard);
        view.effective_places.insert(stocked_source, bandit_camp);
        view.adjacent_places
            .insert(origin, vec![orchard, bandit_camp]);
        view.adjacent_places
            .insert(orchard, vec![origin, bandit_camp]);
        view.adjacent_places
            .insert(bandit_camp, vec![origin, orchard]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![recipe_id]);
        view.workstations
            .insert((origin, WorkstationTag::Mill), vec![mill]);
        view.sources_at
            .insert((orchard, CommodityKind::Firewood), vec![depleted_source]);
        view.sources_at
            .insert((bandit_camp, CommodityKind::Firewood), vec![stocked_source]);
        view.resource_sources.insert(
            depleted_source,
            ResourceSource {
                commodity: CommodityKind::Firewood,
                available_quantity: Quantity(0),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        view.resource_sources.insert(
            stocked_source,
            ResourceSource {
                commodity: CommodityKind::Firewood,
                available_quantity: Quantity(1),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );
        let produce = candidates
            .iter()
            .find(|candidate| candidate.key.kind == GoalKind::ProduceCommodity { recipe_id })
            .expect("reachable stocked source should keep the produce goal emittable");

        assert!(
            produce.evidence_places.contains(&bandit_camp),
            "stocked fallback source should remain in the produce-goal evidence"
        );
        assert!(
            !produce.evidence_places.contains(&orchard),
            "depleted source place should be removed from the produce-goal evidence"
        );
        assert!(
            produce.evidence_entities.contains(&stocked_source),
            "stocked fallback source should remain in the produce-goal evidence entities"
        );
        assert!(
            !produce.evidence_entities.contains(&depleted_source),
            "depleted source entity should be removed from the produce-goal evidence entities"
        );
    }

    #[test]
    fn candidate_evidence_trace_records_resource_source_contributors_and_exclusions() {
        let agent = entity(1);
        let origin = entity(10);
        let orchard = entity(11);
        let bandit_camp = entity(12);
        let mill = entity(20);
        let depleted_source = entity(21);
        let stocked_source = entity(22);
        let recipe_id = RecipeId(0);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([agent, mill, depleted_source, stocked_source]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(mill, EntityKind::Facility);
        view.entity_kinds
            .insert(depleted_source, EntityKind::Facility);
        view.entity_kinds
            .insert(stocked_source, EntityKind::Facility);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(mill, origin);
        view.effective_places.insert(depleted_source, orchard);
        view.effective_places.insert(stocked_source, bandit_camp);
        view.adjacent_places
            .insert(origin, vec![orchard, bandit_camp]);
        view.adjacent_places
            .insert(orchard, vec![origin, bandit_camp]);
        view.adjacent_places
            .insert(bandit_camp, vec![origin, orchard]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![recipe_id]);
        view.workstations
            .insert((origin, WorkstationTag::Mill), vec![mill]);
        view.sources_at
            .insert((orchard, CommodityKind::Firewood), vec![depleted_source]);
        view.sources_at
            .insert((bandit_camp, CommodityKind::Firewood), vec![stocked_source]);
        view.resource_sources.insert(
            depleted_source,
            ResourceSource {
                commodity: CommodityKind::Firewood,
                available_quantity: Quantity(0),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        view.resource_sources.insert(
            stocked_source,
            ResourceSource {
                commodity: CommodityKind::Firewood,
                available_quantity: Quantity(1),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &recipes,
            Tick(5),
            6,
            false,
        );
        let trace = evidence_trace_for_goal(
            &result.diagnostics,
            GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
        );

        assert!(trace.contributors.iter().any(|contributor| {
            contributor.kind == crate::CandidateEvidenceKind::RecipeWorkstation
                && contributor.place == origin
                && contributor.entity == mill
        }));
        assert!(trace.contributors.iter().any(|contributor| {
            contributor.kind == crate::CandidateEvidenceKind::ResourceSource
                && contributor.place == bandit_camp
                && contributor.entity == stocked_source
        }));
        assert!(trace.exclusions.iter().any(|exclusion| {
            exclusion.kind == crate::CandidateEvidenceKind::ResourceSource
                && exclusion.place == orchard
                && exclusion.entity == depleted_source
                && exclusion.reason
                    == crate::CandidateEvidenceExclusionReason::DepletedResourceSource
        }));
    }

    #[test]
    fn missing_recipe_input_without_workstation_withholds_produce_goal() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let recipe_id = RecipeId(0);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![recipe_id]);
        view.sellers
            .insert((place, CommodityKind::Firewood), vec![seller]);

        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::ProduceCommodity { recipe_id }
        ));
    }

    #[test]
    fn missing_recipe_input_reachable_via_known_subrecipe_emits_produce_goal() {
        let agent = entity(1);
        let place = entity(10);
        let mill = entity(20);
        let grain_source = entity(21);
        let bread_recipe_id = RecipeId(0);
        let grain_recipe_id = RecipeId(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(mill, EntityKind::Facility);
        view.entity_kinds.insert(grain_source, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(mill, place);
        view.effective_places.insert(grain_source, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes
            .insert(agent, vec![bread_recipe_id, grain_recipe_id]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::Mill), vec![mill]);
        view.workstations
            .insert((place, WorkstationTag::OrchardRow), vec![grain_source]);
        view.resource_sources.insert(
            grain_source,
            ResourceSource {
                commodity: CommodityKind::Grain,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Bread, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(2))],
            WorkstationTag::Mill,
        ));
        recipes.register(RecipeDefinition {
            name: "Harvest Grain".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Grain, Quantity(2))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity {
                recipe_id: bread_recipe_id
            }
        ));
    }

    #[test]
    fn restock_requires_profile_demand_gap_and_replenishment_path() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.effective_places.insert(agent, place);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place,
                tick: Tick(2),
                counterparty: Some(seller),
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        );
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }
        ));
    }

    #[test]
    fn enterprise_emitters_use_precomputed_restock_signals() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let workstation = entity(3);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.effective_places.insert(agent, place);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place,
                tick: Tick(2),
                counterparty: Some(seller),
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        );
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::Mill), vec![workstation]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Bread, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(2))],
            WorkstationTag::Mill,
        ));
        let blocked = BlockedIntentMemory::default();

        let ctx = GenerationContext {
            view: &view,
            agent,
            place: Some(place),
            travel_horizon: 6,
            enterprise: EnterpriseSignals::default(),
            blocked: &blocked,
            violation_memory: &ViolationMemory::default(),
            recipes: &recipes,
            current_tick: Tick(5),
            tracing_enabled: false,
        };
        let mut candidates = Vec::new();
        let mut diagnostics = CandidateGenerationDiagnostics::default();

        emit_restock_goals(&mut candidates, &mut diagnostics, &ctx);
        emit_produce_goals(&mut candidates, &mut diagnostics, &ctx, None, None);
        assert!(!contains_goal(
            &candidates,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }
        ));

        let ctx = GenerationContext {
            enterprise: analyze_candidate_enterprise(&view, agent, Some(place)),
            ..ctx
        };
        let mut candidates = Vec::new();
        let mut diagnostics = CandidateGenerationDiagnostics::default();

        emit_restock_goals(&mut candidates, &mut diagnostics, &ctx);
        emit_produce_goals(&mut candidates, &mut diagnostics, &ctx, None, None);

        assert!(contains_goal(
            &candidates,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0)
            }
        ));
    }

    #[test]
    fn local_corpse_with_possessions_emits_loot_goal() {
        let agent = entity(1);
        let place = entity(10);
        let corpse = entity(2);
        let bread = entity(3);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.direct_possessions.insert(corpse, vec![bread]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(&candidates, GoalKind::LootCorpse { corpse }));
    }

    #[test]
    fn local_corpse_with_believed_inventory_emits_loot_goal() {
        let agent = entity(1);
        let place = entity(10);
        let corpse = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.commodity_quantities
            .insert((corpse, CommodityKind::Coin), Quantity(5));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(&candidates, GoalKind::LootCorpse { corpse }));
    }

    #[test]
    fn local_corpse_with_believed_inventory_emits_acquire_commodity() {
        let agent = entity(1);
        let place = entity(10);
        let corpse = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.commodity_quantities
            .insert((corpse, CommodityKind::Bread), Quantity(2));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn local_corpse_without_matching_believed_inventory_does_not_emit_acquire_commodity() {
        let agent = entity(1);
        let place = entity(10);
        let corpse = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.commodity_quantities
            .insert((corpse, CommodityKind::Coin), Quantity(5));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
    }

    #[test]
    fn sell_commodity_still_not_emitted_before_s04() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.homeostatic_needs.insert(agent, fatigue(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!candidates
            .iter()
            .any(|candidate| { matches!(candidate.key.kind, GoalKind::SellCommodity { .. }) }));
    }

    #[test]
    fn merchant_with_stock_and_demand_still_does_not_emit_sell_commodity_before_s04() {
        let agent = entity(1);
        let place = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, place, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(place, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(bread, place);
        view.entities_at.insert(place, vec![agent, bread]);
        view.direct_possessions.insert(agent, vec![bread]);
        view.direct_possessors.insert(bread, agent);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((agent, CommodityKind::Bread), Quantity(3));
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((agent, bread));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(1),
                place,
                tick: Tick(2),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            }
        ));
    }

    #[test]
    fn local_corpse_with_grave_plot_emits_bury_goal() {
        let agent = entity(1);
        let place = entity(10);
        let corpse = entity(2);
        let grave_plot = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.entity_kinds.insert(grave_plot, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.effective_places.insert(grave_plot, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.workstations
            .insert((place, WorkstationTag::GravePlot), vec![grave_plot]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::BuryCorpse {
                corpse,
                burial_site: grave_plot,
            }
        ));
    }

    #[test]
    fn local_owned_item_emits_theft_goal() {
        let agent = entity(1);
        let owner = entity(2);
        let place = entity(10);
        let item = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, owner]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(owner, EntityKind::Agent);
        view.entity_kinds.insert(item, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(owner, place);
        view.effective_places.insert(item, place);
        view.entities_at.insert(place, vec![agent, owner, item]);
        view.entity_loads.insert(agent, LoadUnits(1));
        view.carry_capacities.insert(agent, LoadUnits(5));
        view.entity_loads.insert(item, LoadUnits(2));
        view.theft_disposition_profiles.insert(
            agent,
            worldwake_core::TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(400),
                witness_risk_penalty: pm(100),
            },
        );
        view.believed_owners.insert(item, owner);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::StealItem { target_item: item }
        ));
    }

    #[test]
    fn theft_candidate_respects_preconditions_and_witness_gate() {
        let agent = entity(1);
        let owner = entity(2);
        let observer_a = entity(3);
        let observer_b = entity(4);
        let observer_c = entity(5);
        let place = entity(10);
        let valid_item = entity(20);
        let self_owned = entity(21);
        let unowned = entity(22);
        let controllable = entity(23);
        let possessed = entity(24);
        let contained = entity(25);
        let too_heavy = entity(26);

        let mut view = TestBeliefView::default();
        view.alive
            .extend([agent, owner, observer_a, observer_b, observer_c]);
        for actor in [agent, owner, observer_a, observer_b, observer_c] {
            view.entity_kinds.insert(actor, EntityKind::Agent);
            view.effective_places.insert(actor, place);
        }
        for item in [
            valid_item,
            self_owned,
            unowned,
            controllable,
            possessed,
            contained,
            too_heavy,
        ] {
            view.entity_kinds.insert(item, EntityKind::ItemLot);
            view.effective_places.insert(item, place);
        }
        view.entities_at.insert(
            place,
            vec![
                agent,
                owner,
                observer_a,
                observer_b,
                valid_item,
                self_owned,
                unowned,
                controllable,
                possessed,
                contained,
                too_heavy,
            ],
        );
        view.entity_loads.insert(agent, LoadUnits(1));
        view.carry_capacities.insert(agent, LoadUnits(5));
        for item in [
            valid_item,
            self_owned,
            unowned,
            controllable,
            possessed,
            contained,
        ] {
            view.entity_loads.insert(item, LoadUnits(2));
        }
        view.entity_loads.insert(too_heavy, LoadUnits(10));
        view.theft_disposition_profiles.insert(
            agent,
            worldwake_core::TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(400),
                witness_risk_penalty: pm(100),
            },
        );
        view.believed_owners.insert(valid_item, owner);
        view.believed_owners.insert(self_owned, agent);
        view.believed_owners.insert(controllable, owner);
        view.believed_owners.insert(possessed, owner);
        view.believed_owners.insert(contained, owner);
        view.believed_owners.insert(too_heavy, owner);
        view.controllable.insert((agent, controllable));
        view.direct_possessors.insert(possessed, owner);
        view.direct_containers.insert(contained, entity(99));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::StealItem {
                target_item: valid_item
            }
        ));
        for rejected in [
            self_owned,
            unowned,
            controllable,
            possessed,
            contained,
            too_heavy,
        ] {
            assert!(
                !contains_goal(
                    &candidates,
                    GoalKind::StealItem {
                        target_item: rejected
                    }
                ),
                "unexpected theft goal for rejected item {rejected:?}"
            );
        }

        view.entities_at.insert(
            place,
            vec![agent, owner, observer_a, observer_b, observer_c, valid_item],
        );
        let witness_blocked = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(
            !contains_goal(
                &witness_blocked,
                GoalKind::StealItem {
                    target_item: valid_item
                }
            ),
            "witness deterrence should suppress theft when motive gate reaches zero"
        );

        view.theft_disposition_profiles.clear();
        let profileless = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(
            !contains_goal(
                &profileless,
                GoalKind::StealItem {
                    target_item: valid_item
                }
            ),
            "agents without TheftDispositionProfile should not emit theft candidates"
        );
    }

    #[test]
    fn patrolling_guard_only_deterrs_theft_when_locally_observed() {
        let agent = entity(1);
        let owner = entity(2);
        let local_guard = entity(3);
        let remote_guard = entity(4);
        let place = entity(10);
        let remote_place = entity(11);
        let item = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, owner, local_guard, remote_guard]);
        for actor in [agent, owner, local_guard, remote_guard] {
            view.entity_kinds.insert(actor, EntityKind::Agent);
        }
        view.entity_kinds.insert(item, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(item, place);
        view.effective_places.insert(owner, remote_place);
        view.effective_places.insert(remote_guard, remote_place);
        view.entities_at.insert(place, vec![agent, item]);
        view.entities_at.insert(remote_place, vec![owner, remote_guard]);
        view.entity_loads.insert(agent, LoadUnits(1));
        view.entity_loads.insert(item, LoadUnits(2));
        view.carry_capacities.insert(agent, LoadUnits(5));
        view.theft_disposition_profiles.insert(
            agent,
            worldwake_core::TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(300),
                witness_risk_penalty: pm(300),
            },
        );
        view.believed_owners.insert(item, owner);
        view.patrol_profiles.insert(local_guard, patrol_profile(400));
        view.patrol_routes.insert(
            local_guard,
            PatrolRoute {
                assigned_places: vec![place],
                current_index: 0,
            },
        );
        view.patrol_profiles.insert(remote_guard, patrol_profile(400));
        view.patrol_routes.insert(
            remote_guard,
            PatrolRoute {
                assigned_places: vec![remote_place],
                current_index: 0,
            },
        );

        let remote_guard_only = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(
            contains_goal(
                &remote_guard_only,
                GoalKind::StealItem { target_item: item }
            ),
            "remote patrolling guards must not suppress theft without local observation"
        );

        view.effective_places.insert(local_guard, place);
        view.entities_at.insert(place, vec![agent, item, local_guard]);

        let local_guard_present = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(
            !contains_goal(
                &local_guard_present,
                GoalKind::StealItem { target_item: item }
            ),
            "a patrolling guard should deter theft only through same-place witness presence"
        );
    }

    #[test]
    fn theft_candidate_knowledge_path_records_direct_local_observation() {
        let agent = entity(1);
        let owner = entity(2);
        let place = entity(10);
        let item = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, owner]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(owner, EntityKind::Agent);
        view.entity_kinds.insert(item, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(owner, place);
        view.effective_places.insert(item, place);
        view.entities_at.insert(place, vec![agent, owner, item]);
        view.entity_loads.insert(agent, LoadUnits(1));
        view.carry_capacities.insert(agent, LoadUnits(5));
        view.entity_loads.insert(item, LoadUnits(2));
        view.theft_disposition_profiles.insert(
            agent,
            worldwake_core::TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(500),
                witness_risk_penalty: pm(100),
            },
        );
        view.believed_owners.insert(item, owner);
        view.beliefs.insert(agent, vec![known_entity(item, place)]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let goal = GoalKey::from(GoalKind::StealItem { target_item: item });
        let trace = evidence_trace_for_goal(&result.diagnostics, goal);
        assert!(
            trace.knowledge_path.entity_beliefs.contains(
                &crate::knowledge_path::BeliefProvenance {
                    subject: item,
                    aspect: BeliefAspect::LocationAt { place },
                    source: PerceptionSource::DirectObservation,
                    observed_tick: Tick(5),
                }
            ),
            "theft candidate should record direct local observation provenance, got {:?}",
            trace.knowledge_path.entity_beliefs
        );
    }

    #[test]
    fn justice_candidates_emit_accuse_from_matching_typed_theft_testimony() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(4);
        let crime_register = entity(5);
        let place = entity(10);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Bread,
            quantity: Quantity(2),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.entity_kinds.insert(crime_register, EntityKind::Record);
        view.record_data.insert(
            crime_register,
            crime_register_record(
                office,
                place,
                RecordEntryId(0),
                InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(agent),
                    effective_tick: Tick(3),
                },
            ),
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.beliefs
            .insert(agent, vec![known_entity(crime_register, place)]);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.social_observations.insert(
            agent,
            vec![SocialObservation {
                detail: SocialObservationDetail::SuspectedTheft {
                    theft,
                    suspect: Some(accused),
                },
                place,
                observed_tick: Tick(3),
                source: PerceptionSource::Report {
                    from: entity(3),
                    chain_len: 1,
                },
            }],
        );

        let mut violations = ViolationMemory::default();
        let violation_id = violations.record(
            ViolationKind::SuspectedTheft {
                theft,
                suspect: None,
            },
            Tick(2),
            50,
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &violations,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::Accuse {
                crime_register,
                accused,
                violation_id,
            }
        ));
    }

    #[test]
    fn justice_candidates_suppress_duplicate_accusation_when_case_already_known() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(4);
        let crime_register = entity(5);
        let place = entity(10);
        let violation_id = worldwake_core::ViolationId(4);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Bread,
            quantity: Quantity(2),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.entity_kinds.insert(crime_register, EntityKind::Record);
        view.record_data.insert(
            crime_register,
            crime_register_record(
                office,
                place,
                RecordEntryId(0),
                InstitutionalClaim::Accusation {
                    accuser: entity(9),
                    accused,
                    violation_id,
                    theft,
                    effective_tick: Tick(4),
                },
            ),
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.beliefs
            .insert(agent, vec![known_entity(crime_register, place)]);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim: InstitutionalClaim::Accusation {
                    accuser: entity(9),
                    accused,
                    violation_id,
                    theft,
                    effective_tick: Tick(4),
                },
                source: InstitutionalKnowledgeSource::Report {
                    from: entity(8),
                    chain_len: 1,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let mut violations = ViolationMemory::default();
        violations.record(
            ViolationKind::SuspectedTheft {
                theft,
                suspect: Some(accused),
            },
            Tick(2),
            50,
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &violations,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            !contains_goal(
                &result.candidates,
                GoalKind::Accuse {
                    crime_register,
                    accused,
                    violation_id,
                }
            ),
            "known current crime case should suppress duplicate accusation candidate"
        );
    }

    #[test]
    fn justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id(
    ) {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(4);
        let crime_register = entity(5);
        let place = entity(10);
        let recorded_violation_id = worldwake_core::ViolationId(4);
        let incoming_violation_id = worldwake_core::ViolationId(0);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Bread,
            quantity: Quantity(2),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.entity_kinds.insert(crime_register, EntityKind::Record);
        view.record_data.insert(
            crime_register,
            crime_register_record(
                office,
                place,
                RecordEntryId(0),
                InstitutionalClaim::Accusation {
                    accuser: entity(9),
                    accused,
                    violation_id: recorded_violation_id,
                    theft,
                    effective_tick: Tick(4),
                },
            ),
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.beliefs
            .insert(agent, vec![known_entity(crime_register, place)]);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.social_observations.insert(
            agent,
            vec![SocialObservation {
                detail: SocialObservationDetail::SuspectedTheft {
                    theft,
                    suspect: Some(accused),
                },
                place,
                observed_tick: Tick(5),
                source: PerceptionSource::Report {
                    from: entity(8),
                    chain_len: 1,
                },
            }],
        );

        let mut violations = ViolationMemory::default();
        let actual_violation_id = violations.record(
            ViolationKind::SuspectedTheft {
                theft,
                suspect: None,
            },
            Tick(5),
            50,
        );
        assert_eq!(
            actual_violation_id, incoming_violation_id,
            "test setup should use a distinct incoming violation id"
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &violations,
            &RecipeRegistry::new(),
            Tick(6),
            6,
            false,
        );

        assert!(
            !contains_goal(
                &result.candidates,
                GoalKind::Accuse {
                    crime_register,
                    accused,
                    violation_id: incoming_violation_id,
                }
            ),
            "recorded accusation for the same theft facts should suppress a second accuse candidate even when the incoming evidence uses a different violation id"
        );
    }

    #[test]
    fn justice_candidates_emit_fine_punishment_from_consulted_accusation() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let record = entity(4);
        let place = entity(10);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(7);
        let violation_id = worldwake_core::ViolationId(5);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(accused, place);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.record_data.insert(
            record,
            crime_register_record(office, place, accusation_entry, claim),
        );
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        view.locally_observed_commodity_quantities
            .insert((agent, accused, CommodityKind::Coin), Quantity(10));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::PunishAccused {
                office,
                accused,
                accusation_entry,
                punishment: worldwake_core::PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(4),
                },
            }
        ));
    }

    #[test]
    fn justice_fine_candidate_trace_records_concrete_selection_provenance() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let record = entity(4);
        let place = entity(10);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(17);
        let violation_id = worldwake_core::ViolationId(5);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(accused, place);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.record_data.insert(
            record,
            crime_register_record(office, place, accusation_entry, claim),
        );
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        view.locally_observed_commodity_quantities
            .insert((agent, accused, CommodityKind::Coin), Quantity(10));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let goal = GoalKey::from(GoalKind::PunishAccused {
            office,
            accused,
            accusation_entry,
            punishment: worldwake_core::PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(4),
            },
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, goal);
        assert_eq!(
            trace.legality,
            Some(
                crate::decision_trace::CandidateLegalityTrace::PunishmentFineSelection(
                    PunishmentFineSelectionTrace {
                        facts: PunishmentFineTraceFacts {
                            office,
                            accusation_entry,
                            accused,
                            theft,
                            actor_place: Some(place),
                            accused_place: Some(place),
                            required_amount: Quantity(4),
                        },
                        locally_observed_quantity: Quantity(10),
                    },
                ),
            ),
            "fine punishment trace should record the concrete planner-visible read"
        );
    }

    #[test]
    fn justice_candidates_fall_back_to_exile_when_fine_is_not_believed_affordable() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let record = entity(4);
        let place = entity(10);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(8);
        let violation_id = worldwake_core::ViolationId(6);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(accused, place);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.record_data.insert(
            record,
            crime_register_record(office, place, accusation_entry, claim),
        );
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        view.factions_by_member.insert(accused, vec![faction]);
        view.commodity_quantities
            .insert((accused, CommodityKind::Coin), Quantity(1));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::PunishAccused {
                office,
                accused,
                accusation_entry,
                punishment: worldwake_core::PunishmentKind::Exile {
                    from_faction: faction
                },
            }
        ));
    }

    #[test]
    fn justice_candidates_fall_back_to_exile_when_fine_is_not_locally_collectible() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let record = entity(4);
        let place = entity(10);
        let remote_place = entity(12);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(9);
        let violation_id = worldwake_core::ViolationId(7);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(accused, remote_place);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.record_data.insert(
            record,
            crime_register_record(office, place, accusation_entry, claim),
        );
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        view.commodity_quantities
            .insert((accused, CommodityKind::Coin), Quantity(10));
        view.factions_by_member.insert(accused, vec![faction]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::PunishAccused {
                office,
                accused,
                accusation_entry,
                punishment: worldwake_core::PunishmentKind::Exile {
                    from_faction: faction
                },
            }
        ));
        assert!(
            !contains_goal(
                &result.candidates,
                GoalKind::PunishAccused {
                    office,
                    accused,
                    accusation_entry,
                    punishment: worldwake_core::PunishmentKind::Fine {
                        commodity: CommodityKind::Coin,
                        amount: Quantity(4),
                    },
                }
            ),
            "planner should not choose Fine from remote inventory belief alone"
        );
    }

    #[test]
    fn justice_candidates_do_not_emit_punishment_from_report_only_case_knowledge() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let place = entity(10);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(9);
        let violation_id = worldwake_core::ViolationId(7);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::Report {
                    from: entity(12),
                    chain_len: 1,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            !contains_goal(
                &result.candidates,
                GoalKind::PunishAccused {
                    office,
                    accused,
                    accusation_entry,
                    punishment: worldwake_core::PunishmentKind::Fine {
                        commodity: CommodityKind::Coin,
                        amount: Quantity(4),
                    },
                }
            ),
            "report-only crime knowledge should not synthesize punishable consulted case targets"
        );
    }

    #[test]
    fn justice_candidates_withhold_punishment_when_no_lawful_fine_or_exile_binding_exists() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let record = entity(4);
        let place = entity(10);
        let faction = entity(11);
        let accusation_entry = RecordEntryId(10);
        let violation_id = worldwake_core::ViolationId(8);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: place,
            commodity: CommodityKind::Coin,
            quantity: Quantity(8),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id,
            theft,
            effective_tick: Tick(3),
        };

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, accused]);
        view.justice_disposition_profiles
            .insert(agent, default_justice_profile());
        view.office_data
            .insert(office, vacant_office("Magistrate", place, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.record_data.insert(
            record,
            crime_register_record(office, place, accusation_entry, claim),
        );
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::CrimeCase {
                    accused,
                    violation_id,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );
        view.commodity_quantities
            .insert((accused, CommodityKind::Coin), Quantity(1));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            !result.candidates.iter().any(|candidate| matches!(
                candidate.key.kind,
                GoalKind::PunishAccused {
                    accused: goal_accused,
                    ..
                } if goal_accused == accused
            )),
            "punishment should be withheld when neither fine nor exile can be bound lawfully"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects() {
        let speaker = entity(1);
        let listener_a = entity(2);
        let listener_b = entity(3);
        let dead_listener = entity(4);
        let crate_lot = entity(5);
        let subject_a = entity(20);
        let subject_b = entity(21);
        let too_deep = entity(22);
        let place = entity(10);
        let remote_a = entity(11);
        let remote_b = entity(12);
        let remote_c = entity(13);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([speaker, listener_a, listener_b, crate_lot]);
        view.dead.insert(dead_listener);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener_a, EntityKind::Agent);
        view.entity_kinds.insert(listener_b, EntityKind::Agent);
        view.entity_kinds.insert(dead_listener, EntityKind::Agent);
        view.entity_kinds.insert(subject_a, EntityKind::Agent);
        view.entity_kinds.insert(subject_b, EntityKind::Agent);
        view.entity_kinds.insert(too_deep, EntityKind::Agent);
        view.entity_kinds.insert(crate_lot, EntityKind::ItemLot);
        view.effective_places.insert(speaker, place);
        view.effective_places.insert(subject_a, remote_a);
        view.effective_places.insert(subject_b, remote_b);
        view.effective_places.insert(too_deep, remote_c);
        view.entities_at.insert(
            place,
            vec![speaker, listener_a, listener_b, dead_listener, crate_lot],
        );
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 2,
                max_relay_chain_len: 2,
                acceptance_fidelity: pm(800),
                ..TellProfile::default()
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                known_entity(subject_a, place),
                (
                    subject_b,
                    believed_state(
                        9,
                        PerceptionSource::Report {
                            from: listener_a,
                            chain_len: 2,
                        },
                    ),
                ),
                (
                    too_deep,
                    believed_state(10, PerceptionSource::Rumor { chain_len: 3 }),
                ),
            ],
        );

        let candidates = generate_candidates(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_a,
                topic: TellTopic::EntityBelief { subject: subject_b },
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_a,
                topic: TellTopic::EntityBelief { subject: subject_a },
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_b,
                topic: TellTopic::EntityBelief { subject: subject_b },
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_b,
                topic: TellTopic::EntityBelief { subject: subject_a },
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: dead_listener,
                topic: TellTopic::EntityBelief { subject: subject_b },
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_a,
                topic: TellTopic::EntityBelief { subject: too_deep },
            }
        ));
    }

    #[test]
    fn social_candidates_require_tell_profile_and_respect_blocked_memory() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([speaker, listener]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.effective_places.insert(speaker, place);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.beliefs.insert(
            speaker,
            vec![(
                subject,
                believed_state(8, PerceptionSource::DirectObservation),
            )],
        );

        let none = generate_candidates(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
        );
        assert!(!contains_goal(
            &none,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));

        view.tell_profiles.insert(speaker, TellProfile::default());
        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: GoalKey::from(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                }),
                place: None,
                target: None,
                action_def: None,
            },
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(10),
            expires_tick: Tick(20),
        });

        let blocked_candidates =
            generate_candidates(&view, speaker, &blocked, &RecipeRegistry::new(), Tick(11));
        assert!(!contains_goal(
            &blocked_candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
    }

    #[test]
    fn social_candidates_record_direct_observability_omission_reason() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places
            .extend([(speaker, place), (listener, place), (subject, place)]);
        view.entities_at
            .insert(place, vec![speaker, listener, subject]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.beliefs.insert(
            speaker,
            vec![(
                subject,
                believed_state(8, PerceptionSource::DirectObservation),
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
        assert!(contains_social_omission(
            &result.diagnostics,
            listener,
            subject,
            TellTopicOmissionReason::DirectlyObservableByListener,
        ));
    }

    #[test]
    fn social_candidates_emit_institutional_claim_topics_even_when_office_entity_is_visible() {
        let speaker = entity(1);
        let listener = entity(2);
        let office = entity(20);
        let place = entity(10);
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(8),
        };
        let mut view = TestBeliefView::default();
        view.alive.extend([speaker, listener, office]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places
            .extend([(speaker, place), (listener, place), (office, place)]);
        view.entities_at
            .insert(place, vec![speaker, listener, office]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.institutional_claims.insert(
            (speaker, InstitutionalBeliefKey::OfficeHolderOf { office }),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(8),
                learned_at: Some(place),
            }],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::InstitutionalClaim { claim },
            }
        ));
    }

    #[test]
    fn social_candidates_suppress_unchanged_repeat_tells_via_told_memory() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView {
            current_tick: Tick(11),
            ..Default::default()
        };
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.extend([
            (speaker, place),
            (listener, place),
            (subject, remote_place),
        ]);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        let belief = known_entity(subject, place).1;
        view.beliefs
            .insert(speaker, vec![(subject, belief.clone())]);
        view.told_beliefs
            .insert(speaker, vec![told_memory(listener, subject, 10, &belief)]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
        assert!(contains_social_omission(
            &result.diagnostics,
            listener,
            subject,
            TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        ));
    }

    #[test]
    fn social_candidates_reemit_when_shared_content_changes() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView {
            current_tick: Tick(11),
            ..Default::default()
        };
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.extend([
            (speaker, place),
            (listener, place),
            (subject, remote_place),
        ]);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        let old_belief = known_entity(subject, place).1;
        let mut new_belief = old_belief.clone();
        new_belief
            .last_known_inventory
            .insert(CommodityKind::Bread, Quantity(2));
        view.beliefs
            .insert(speaker, vec![(subject, new_belief.clone())]);
        view.told_beliefs.insert(
            speaker,
            vec![told_memory(listener, subject, 10, &old_belief)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
        assert!(!contains_social_omission(
            &result.diagnostics,
            listener,
            subject,
            TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        ));
    }

    #[test]
    fn social_candidates_ignore_observed_tick_only_refreshes() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView {
            current_tick: Tick(11),
            ..Default::default()
        };
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.extend([
            (speaker, place),
            (listener, place),
            (subject, remote_place),
        ]);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        let old_belief = known_entity(subject, place).1;
        let mut refreshed_belief = old_belief.clone();
        refreshed_belief.observed_tick = Tick(11);
        view.beliefs
            .insert(speaker, vec![(subject, refreshed_belief)]);
        view.told_beliefs.insert(
            speaker,
            vec![told_memory(listener, subject, 10, &old_belief)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
        assert!(contains_social_omission(
            &result.diagnostics,
            listener,
            subject,
            TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        ));
    }

    #[test]
    fn social_candidates_reemit_when_tell_memory_has_expired() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(20);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView {
            current_tick: Tick(60),
            ..Default::default()
        };
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.extend([
            (speaker, place),
            (listener, place),
            (subject, remote_place),
        ]);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        let belief = known_entity(subject, place).1;
        view.beliefs
            .insert(speaker, vec![(subject, belief.clone())]);
        view.told_beliefs
            .insert(speaker, vec![told_memory(listener, subject, 1, &belief)]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(60),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }
        ));
        assert!(!contains_social_omission(
            &result.diagnostics,
            listener,
            subject,
            TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
        ));
    }

    #[test]
    fn social_candidates_listener_aware_filtering_happens_before_truncation() {
        let speaker = entity(1);
        let listener = entity(2);
        let recent_subject = entity(20);
        let older_subject = entity(21);
        let place = entity(10);
        let recent_place = entity(11);
        let older_place = entity(12);
        let mut view = TestBeliefView {
            current_tick: Tick(11),
            ..Default::default()
        };
        view.alive
            .extend([speaker, listener, recent_subject, older_subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(recent_subject, EntityKind::Agent);
        view.entity_kinds.insert(older_subject, EntityKind::Agent);
        view.effective_places.extend([
            (speaker, place),
            (listener, place),
            (recent_subject, recent_place),
            (older_subject, older_place),
        ]);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 1,
                ..TellProfile::default()
            },
        );
        let recent_belief = known_entity(recent_subject, place).1;
        let mut older_belief = known_entity(older_subject, place).1;
        older_belief.observed_tick = Tick(8);
        view.beliefs.insert(
            speaker,
            vec![
                (recent_subject, recent_belief.clone()),
                (older_subject, older_belief),
            ],
        );
        view.told_beliefs.insert(
            speaker,
            vec![told_memory(listener, recent_subject, 10, &recent_belief)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(11),
            6,
            false,
        );

        assert!(!contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief {
                    subject: recent_subject,
                },
            }
        ));
        assert!(contains_goal(
            &result.candidates,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief {
                    subject: older_subject,
                },
            }
        ));
    }

    #[test]
    fn cargo_candidate_emitted_from_local_stock_and_demand() {
        let agent = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, origin, destination, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(origin, EntityKind::Place);
        view.entity_kinds.insert(destination, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![agent, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((agent, bread));
        view.carry_capacities.insert(agent, LoadUnits(3));
        view.entity_loads.insert(agent, LoadUnits(0));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(destination, CommodityKind::Bread, 2)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        let goal = candidates
            .iter()
            .find(|candidate| {
                candidate.key.kind
                    == GoalKind::MoveCargo {
                        commodity: CommodityKind::Bread,
                        destination,
                    }
            })
            .unwrap();
        assert!(goal.evidence_entities.contains(&bread));
        assert!(goal.evidence_places.contains(&origin));
        assert!(goal.evidence_places.contains(&destination));
    }

    #[test]
    fn no_cargo_candidate_without_local_stock() {
        let agent = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let remote_bread = entity(20);
        let remote_place = entity(12);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([agent, origin, destination, remote_bread, remote_place]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(origin, EntityKind::Place);
        view.entity_kinds.insert(destination, EntityKind::Place);
        view.entity_kinds.insert(remote_place, EntityKind::Place);
        view.entity_kinds.insert(remote_bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(remote_bread, remote_place);
        view.entities_at.insert(origin, vec![agent]);
        view.entities_at.insert(remote_place, vec![remote_bread]);
        view.lot_commodities
            .insert(remote_bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((remote_bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((agent, remote_bread));
        view.carry_capacities.insert(agent, LoadUnits(3));
        view.entity_loads.insert(agent, LoadUnits(0));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(destination, CommodityKind::Bread, 2)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
        ));
    }

    #[test]
    fn no_cargo_candidate_when_at_destination() {
        let agent = entity(1);
        let destination = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, destination, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(destination, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at.insert(destination, vec![agent, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((agent, bread));
        view.carry_capacities.insert(agent, LoadUnits(3));
        view.entity_loads.insert(agent, LoadUnits(0));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(destination, CommodityKind::Bread, 2)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(
            &candidates,
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
        ));
    }

    #[test]
    fn deliverable_quantity_is_capped_by_carry_capacity() {
        let agent = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, origin, destination, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(origin, EntityKind::Place);
        view.entity_kinds.insert(destination, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![agent, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(5));
        view.controllable.insert((agent, bread));
        view.carry_capacities.insert(agent, LoadUnits(2));
        view.entity_loads.insert(agent, LoadUnits(0));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(destination, CommodityKind::Bread, 5)]);

        assert_eq!(
            deliverable_quantity(&view, agent, origin, destination, CommodityKind::Bread),
            Quantity(2)
        );
    }

    #[test]
    fn no_cargo_candidate_when_zero_deliverable() {
        let agent = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, origin, destination, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(origin, EntityKind::Place);
        view.entity_kinds.insert(destination, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![agent, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((agent, bread));
        view.carry_capacities.insert(agent, LoadUnits(1));
        view.entity_loads.insert(agent, LoadUnits(1));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(destination, CommodityKind::Bread, 3)]);

        assert_eq!(
            deliverable_quantity(&view, agent, origin, destination, CommodityKind::Bread),
            Quantity(0)
        );
        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(!contains_goal(
            &candidates,
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
        ));
    }

    #[test]
    fn generate_candidates_orchestrates_all_domain_groups() {
        let agent = entity(1);
        let seller = entity(2);
        let attacker = entity(3);
        let place = entity(10);
        let adjacent = entity(11);
        let workstation = entity(12);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller, attacker]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::Mill), vec![workstation]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place,
                tick: Tick(2),
                counterparty: Some(seller),
                reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
            }],
        );
        view.hostiles.insert(agent, vec![attacker]);
        view.attackers.insert(agent, vec![attacker]);
        view.adjacent_places.insert(place, vec![adjacent]);

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Bread, Quantity(1))],
            vec![(CommodityKind::Grain, Quantity(2))],
            WorkstationTag::Mill,
        ));

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &recipes,
            Tick(5),
        );

        assert!(contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0)
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }
        ));
        assert!(contains_goal(&candidates, GoalKind::ReduceDanger));
    }

    #[test]
    fn political_candidates_emit_claim_and_support_for_visible_vacant_office() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.entities_at.insert(town, vec![agent, candidate]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
        );

        assert!(contains_goal(&candidates, GoalKind::ClaimOffice { office }));
        assert!(contains_goal(
            &candidates,
            GoalKind::SupportCandidateForOffice { office, candidate }
        ));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn political_candidates_use_institutional_beliefs_for_unknown_certain_and_conflicted_reads() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let record = entity(4);
        let town = entity(10);
        let archive = entity(12);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate, record]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.entity_kinds.insert(record, EntityKind::Record);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.effective_places.insert(record, archive);
        view.entities_at.insert(town, vec![agent, candidate]);
        view.entities_at.insert(archive, vec![record]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![
                known_entity(office, town),
                known_entity(candidate, town),
                known_entity(record, archive),
            ],
        );
        view.record_data
            .insert(record, office_register_record(agent, archive, office));

        let unknown_with_record = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            contains_goal(&unknown_with_record.candidates, GoalKind::ClaimOffice { office }),
            "unknown vacancy belief should remain emittable when a consultable office register is known"
        );
        assert!(
            contains_goal(
                &unknown_with_record.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "unknown vacancy belief should still allow support goals when a consultable office register is known"
        );
        for goal in &unknown_with_record.candidates {
            if matches!(
                goal.key.kind,
                GoalKind::ClaimOffice { office: goal_office }
                    | GoalKind::SupportCandidateForOffice { office: goal_office, .. }
                    if goal_office == office
            ) {
                assert!(
                    goal.evidence_entities.contains(&record),
                    "political goals emitted through the consult path must carry record evidence"
                );
                assert!(
                    goal.evidence_places.contains(&archive),
                    "political goals emitted through the consult path must carry record home-place evidence"
                );
            }
        }

        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        let with_certain_vacancy = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );
        assert!(contains_goal(
            &with_certain_vacancy.candidates,
            GoalKind::ClaimOffice { office }
        ));
        assert!(contains_goal(
            &with_certain_vacancy.candidates,
            GoalKind::SupportCandidateForOffice { office, candidate }
        ));

        view.office_holder_beliefs.insert(
            office,
            InstitutionalBeliefRead::Conflicted(vec![None, Some(candidate)]),
        );
        let conflicted = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );
        assert!(
            !contains_goal(&conflicted.candidates, GoalKind::ClaimOffice { office }),
            "conflicted office-holder beliefs must suppress claim-office generation"
        );
        assert!(
            !contains_goal(
                &conflicted.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "conflicted office-holder beliefs must suppress support-office generation"
        );
        assert!(contains_political_omission(
            &conflicted.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted,
        ));
        assert!(contains_political_omission(
            &conflicted.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeHolderBeliefConflicted,
        ));
    }

    #[test]
    fn political_candidates_unknown_belief_require_consultable_record_evidence() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.entities_at.insert(town, vec![agent, candidate]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let no_record = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            !contains_goal(&no_record.candidates, GoalKind::ClaimOffice { office }),
            "unknown vacancy belief without a consultable record must suppress ClaimOffice"
        );
        assert!(
            !contains_goal(
                &no_record.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "unknown vacancy belief without a consultable record must suppress support goals"
        );
        assert!(contains_political_omission(
            &no_record.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeHolderBeliefUnknownNoConsultableRecord,
        ));
        assert!(contains_political_omission(
            &no_record.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeHolderBeliefUnknownNoConsultableRecord,
        ));
    }

    #[test]
    fn political_candidates_do_not_fallback_to_live_support_or_holder_helpers() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let incumbent = entity(4);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate, incumbent]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(incumbent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.effective_places.insert(incumbent, town);
        view.entities_at
            .insert(town, vec![agent, candidate, incumbent]);
        view.office_data
            .insert(office, vacant_office("Captain", town, faction));
        view.office_holders.insert(office, incumbent);
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.support_declarations.insert((agent, office), agent);
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            contains_goal(&result.candidates, GoalKind::ClaimOffice { office }),
            "candidate generation must not fallback to live office-holder or self-support helpers once institutional beliefs are present"
        );
        assert!(
            contains_goal(
                &result.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "support-candidate emission must ignore stale live support declarations when the institutional belief read does not confirm them"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn political_candidates_require_visible_vacancy_and_skip_existing_declaration() {
        let agent = entity(1);
        let office = entity(2);
        let incumbent = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, incumbent]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(incumbent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(incumbent, town);
        view.entities_at.insert(town, vec![agent, incumbent]);
        view.factions_by_member.insert(agent, vec![faction]);
        view.beliefs.insert(agent, vec![known_entity(office, town)]);

        let mut office_data = vacant_office("Captain", town, faction);
        view.office_holders.insert(office, incumbent);
        view.office_data.insert(office, office_data.clone());
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(incumbent)));
        let occupied_with_stale_vacancy = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );
        assert!(!contains_goal(
            &occupied_with_stale_vacancy.candidates,
            GoalKind::ClaimOffice { office }
        ));
        assert!(contains_political_omission(
            &occupied_with_stale_vacancy.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant,
        ));
        assert!(contains_political_omission(
            &occupied_with_stale_vacancy.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant,
        ));

        view.office_holders.clear();
        office_data.vacancy_since = None;
        view.office_data.insert(office, office_data.clone());
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        let filled = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );
        assert!(!contains_goal(
            &filled.candidates,
            GoalKind::ClaimOffice { office }
        ));
        assert!(contains_political_omission(
            &filled.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant,
        ));
        assert!(contains_political_omission(
            &filled.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant,
        ));

        office_data.vacancy_since = Some(Tick(2));
        view.office_data.insert(office, office_data);
        view.support_declarations.insert((agent, office), agent);
        view.support_declaration_beliefs.insert(
            (office, agent),
            InstitutionalBeliefRead::Certain(Some(agent)),
        );
        let declared = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );
        assert!(!contains_goal(
            &declared.candidates,
            GoalKind::ClaimOffice { office }
        ));
        assert!(contains_political_omission(
            &declared.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::AlreadyDeclaredSupport,
        ));
    }

    #[test]
    fn political_candidates_suppress_conflicted_support_beliefs() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.entities_at.insert(town, vec![agent, candidate]);
        view.office_data
            .insert(office, vacant_office("Captain", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.support_declaration_beliefs.insert(
            (office, agent),
            InstitutionalBeliefRead::Conflicted(vec![Some(agent), Some(candidate)]),
        );
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            !contains_goal(&result.candidates, GoalKind::ClaimOffice { office }),
            "conflicted self-support belief should suppress ClaimOffice commitment"
        );
        assert!(
            !contains_goal(
                &result.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "conflicted self-support belief should suppress support commitments"
        );
        assert!(contains_political_omission(
            &result.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::SupportDeclarationBeliefConflicted,
        ));
        assert!(contains_political_omission(
            &result.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            Some(candidate),
            PoliticalCandidateOmissionReason::SupportDeclarationBeliefConflicted,
        ));
    }

    #[test]
    fn political_candidates_emit_claim_for_force_law_offices_and_keep_support_suppressed() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.entities_at.insert(town, vec![agent, candidate]);

        let mut office_data = vacant_office("Warlord", town, faction);
        office_data.succession_law = worldwake_core::SuccessionLaw::Force;
        view.office_data.insert(office, office_data);

        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.force_controller_beliefs
            .insert(office, InstitutionalBeliefRead::Certain((None, false)));
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let candidates = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            contains_goal(&candidates.candidates, GoalKind::ClaimOffice { office }),
            "Force-law offices should emit ClaimOffice when control is believed vacant"
        );
        assert!(
            !contains_goal(
                &candidates.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "Force-law offices should keep support-candidate goals suppressed"
        );
        assert!(contains_political_omission(
            &candidates.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::ForceSuccessionLaw,
        ));
    }

    #[test]
    fn political_candidates_emit_claim_for_enemy_held_force_office() {
        let agent = entity(1);
        let enemy = entity(2);
        let office = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, enemy]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(enemy, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(enemy, town);
        view.entities_at.insert(town, vec![agent, enemy]);

        let mut office_data = vacant_office("Warlord", town, faction);
        office_data.succession_law = worldwake_core::SuccessionLaw::Force;
        view.office_data.insert(office, office_data);
        view.factions_by_member.insert(agent, vec![faction]);
        view.hostiles.insert(agent, vec![enemy]);
        view.force_controller_beliefs.insert(
            office,
            InstitutionalBeliefRead::Certain((Some(enemy), false)),
        );
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(enemy, town)],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(contains_goal(
            &result.candidates,
            GoalKind::ClaimOffice { office }
        ));
        assert!(!contains_political_omission(
            &result.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::ForceSuccessionLaw,
        ));
    }

    #[test]
    fn political_candidates_record_ineligible_actor_and_support_target_omissions() {
        let agent = entity(1);
        let office = entity(2);
        let ineligible_candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let other_faction = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, ineligible_candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds
            .insert(ineligible_candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(ineligible_candidate, town);
        view.entities_at
            .insert(town, vec![agent, ineligible_candidate]);
        view.office_data
            .insert(office, vacant_office("Captain", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![other_faction]);
        view.factions_by_member
            .insert(ineligible_candidate, vec![other_faction]);
        view.loyalties
            .insert((agent, ineligible_candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![
                known_entity(office, town),
                known_entity(ineligible_candidate, town),
            ],
        );

        let candidates = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
            false,
        );

        assert!(
            !contains_goal(&candidates.candidates, GoalKind::ClaimOffice { office }),
            "ineligible actors must not emit ClaimOffice candidates"
        );
        assert!(
            !contains_goal(
                &candidates.candidates,
                GoalKind::SupportCandidateForOffice {
                    office,
                    candidate: ineligible_candidate,
                }
            ),
            "ineligible support targets must not emit support candidates"
        );
        assert!(contains_political_omission(
            &candidates.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::ActorNotEligible,
        ));
        assert!(contains_political_omission(
            &candidates.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
            office,
            Some(ineligible_candidate),
            PoliticalCandidateOmissionReason::CandidateNotEligible,
        ));
    }

    // ── S28-002: Knowledge path instrumentation tests ──

    #[test]
    fn tracing_disabled_produces_empty_knowledge_paths() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false, // tracing DISABLED
        );

        // All evidence traces should have empty knowledge paths.
        for trace in result.diagnostics.evidence.values() {
            assert_eq!(
                trace.knowledge_path,
                KnowledgePath::default(),
                "knowledge_path should be empty when tracing is disabled, but goal {:?} had {:?}",
                trace.opportunity,
                trace.knowledge_path,
            );
        }
    }

    #[test]
    fn need_candidate_knowledge_path_records_self_need() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        // Provide belief provenance for the seller
        view.beliefs.insert(
            agent,
            vec![(
                seller,
                believed_state(3, PerceptionSource::DirectObservation),
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let acquire_key = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, acquire_key);

        assert!(
            trace
                .knowledge_path
                .self_knowledge
                .contains(&SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Hunger,
                    permille: pm(250),
                }),
            "knowledge_path.self_knowledge should contain NeedLevel(Hunger, 250), got {:?}",
            trace.knowledge_path.self_knowledge,
        );
    }

    #[test]
    fn need_candidate_knowledge_path_records_seller_belief() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        // Agent has belief about the seller from a report
        view.beliefs.insert(
            agent,
            vec![(
                seller,
                believed_state(3, PerceptionSource::DirectObservation),
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let acquire_key = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, acquire_key);

        assert!(
            trace.knowledge_path.entity_beliefs.iter().any(|bp| {
                bp.subject == seller
                    && bp.aspect
                        == BeliefAspect::HasCommodity {
                            commodity: CommodityKind::Bread,
                        }
                    && bp.source == PerceptionSource::DirectObservation
                    && bp.observed_tick == Tick(3)
            }),
            "entity_beliefs should contain BeliefProvenance for seller with HasCommodity(Bread), got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn sleep_goal_knowledge_path_records_fatigue() {
        let agent = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.homeostatic_needs.insert(agent, fatigue(600));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let sleep_key = GoalKey::from(GoalKind::Sleep);
        let trace = evidence_trace_for_goal(&result.diagnostics, sleep_key);

        assert!(
            trace
                .knowledge_path
                .self_knowledge
                .contains(&SelfKnowledgeProvenance::NeedLevel {
                    need: HomeostaticNeedId::Fatigue,
                    permille: pm(600),
                }),
            "knowledge_path.self_knowledge should contain NeedLevel(Fatigue, 600), got {:?}",
            trace.knowledge_path.self_knowledge,
        );
    }

    #[test]
    fn produce_candidate_knowledge_path_records_resource_source() {
        let agent = entity(1);
        let workstation = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(workstation, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::OrchardRow), vec![workstation]);
        view.workstation_tags
            .insert(workstation, WorkstationTag::OrchardRow);
        view.resource_sources.insert(
            workstation,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(10),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        // Provide belief provenance for the workstation/resource-source entity
        view.beliefs.insert(
            agent,
            vec![(
                workstation,
                believed_state(3, PerceptionSource::DirectObservation),
            )],
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Apple, Quantity(2))],
            Vec::new(),
            WorkstationTag::OrchardRow,
        ));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &recipes,
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let produce_key = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(0),
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, produce_key);

        assert!(
            trace.knowledge_path.entity_beliefs.iter().any(|bp| {
                bp.subject == workstation
                    && bp.aspect
                        == BeliefAspect::IsResourceSource {
                            commodity: CommodityKind::Apple,
                        }
                    && bp.source == PerceptionSource::DirectObservation
                    && bp.observed_tick == Tick(3)
            }),
            "entity_beliefs should contain BeliefProvenance for resource source with IsResourceSource(Apple), got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn produce_candidate_knowledge_path_records_workstation() {
        let agent = entity(1);
        let workstation = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, workstation]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(workstation, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.known_recipes.insert(agent, vec![RecipeId(0)]);
        view.unique_item_counts
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.workstations
            .insert((place, WorkstationTag::Forge), vec![workstation]);
        view.workstation_tags
            .insert(workstation, WorkstationTag::Forge);
        // Agent is a merchant selling Swords (triggers serves_restock)
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Sword]),
                home_market: Some(place),
            },
        );
        // Demand memory creates the restock gap
        view.demand_memory
            .insert(agent, vec![demand(place, CommodityKind::Sword, 3)]);
        // Crafting recipe: Firewood -> Sword at Forge (has inputs, workstation is NOT resource source)
        view.commodity_quantities
            .insert((agent, CommodityKind::Firewood), Quantity(5));
        view.beliefs.insert(
            agent,
            vec![(
                workstation,
                believed_state(2, PerceptionSource::DirectObservation),
            )],
        );

        let mut recipes = RecipeRegistry::new();
        recipes.register(sample_recipe(
            vec![(CommodityKind::Sword, Quantity(1))],
            vec![(CommodityKind::Firewood, Quantity(2))],
            WorkstationTag::Forge,
        ));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &recipes,
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let produce_key = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(0),
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, produce_key);

        assert!(
            trace.knowledge_path.entity_beliefs.iter().any(|bp| {
                bp.subject == workstation
                    && bp.aspect
                        == BeliefAspect::HasWorkstation {
                            tag: WorkstationTag::Forge,
                        }
                    && bp.source == PerceptionSource::DirectObservation
                    && bp.observed_tick == Tick(2)
            }),
            "entity_beliefs should contain BeliefProvenance for workstation with HasWorkstation(Forge), got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn restock_candidate_knowledge_path_records_merchant_identity() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );
        // Demand memory creates the restock gap
        view.demand_memory
            .insert(agent, vec![demand(place, CommodityKind::Bread, 5)]);
        // Seller has bread for sale
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        view.beliefs.insert(
            agent,
            vec![(
                seller,
                believed_state(4, PerceptionSource::DirectObservation),
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true, // tracing ENABLED
        );

        let restock_key = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, restock_key);

        assert!(
            trace
                .knowledge_path
                .self_knowledge
                .contains(&SelfKnowledgeProvenance::MerchantIdentity),
            "knowledge_path.self_knowledge should contain MerchantIdentity, got {:?}",
            trace.knowledge_path.self_knowledge,
        );
    }

    #[test]
    fn engage_hostile_knowledge_path_records_hostile_belief() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, hostile]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(hostile, place);
        view.entities_at.insert(place, vec![agent, hostile]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![hostile]);
        view.beliefs.insert(
            agent,
            vec![(
                hostile,
                believed_state(3, PerceptionSource::DirectObservation),
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::EngageHostile { target: hostile });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace
                .knowledge_path
                .entity_beliefs
                .contains(&crate::knowledge_path::BeliefProvenance {
                    subject: hostile,
                    aspect: BeliefAspect::Hostile,
                    source: PerceptionSource::DirectObservation,
                    observed_tick: Tick(3),
                }),
            "knowledge_path.entity_beliefs should contain Hostile belief for hostile target, got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn reduce_danger_knowledge_path_records_own_wounds() {
        let agent = entity(1);
        let attacker = entity(2);
        let place = entity(10);
        let adjacent = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, attacker]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(attacker, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(attacker, place);
        view.entities_at.insert(place, vec![agent, attacker]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.wounds.insert(agent, vec![wound(1), wound(2)]);
        view.attackers.insert(agent, vec![attacker]);
        view.adjacent_places.insert(place, vec![adjacent]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::ReduceDanger);
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace
                .knowledge_path
                .self_knowledge
                .contains(&SelfKnowledgeProvenance::OwnWounds { count: 2 }),
            "knowledge_path.self_knowledge should contain OwnWounds {{ count: 2 }}, got {:?}",
            trace.knowledge_path.self_knowledge,
        );
    }

    #[test]
    fn care_knowledge_path_records_wounded_belief() {
        let agent = entity(1);
        let patient = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(patient, place);
        view.entities_at.insert(place, vec![agent, patient]);
        view.beliefs.insert(
            agent,
            vec![(
                patient,
                BelievedEntityState {
                    wounds: vec![wound(1)],
                    alive: true,
                    ..believed_state(4, PerceptionSource::DirectObservation)
                },
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::TreatWounds { patient });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace.knowledge_path.entity_beliefs.contains(
                &crate::knowledge_path::BeliefProvenance {
                    subject: patient,
                    aspect: BeliefAspect::Wounded,
                    source: PerceptionSource::DirectObservation,
                    observed_tick: Tick(4),
                }
            ),
            "knowledge_path.entity_beliefs should contain Wounded belief for patient, got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn loot_knowledge_path_records_corpse_belief() {
        let agent = entity(1);
        let corpse = entity(2);
        let bread = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.dead.insert(corpse);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(corpse, place);
        view.corpses_at.insert(place, vec![corpse]);
        view.direct_possessions.insert(corpse, vec![bread]);
        view.beliefs.insert(
            agent,
            vec![(
                corpse,
                BelievedEntityState {
                    alive: false,
                    ..believed_state(2, PerceptionSource::DirectObservation)
                },
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::LootCorpse { corpse });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace.knowledge_path.entity_beliefs.contains(
                &crate::knowledge_path::BeliefProvenance {
                    subject: corpse,
                    aspect: BeliefAspect::Dead,
                    source: PerceptionSource::DirectObservation,
                    observed_tick: Tick(2),
                }
            ),
            "knowledge_path.entity_beliefs should contain Dead belief for corpse, got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn institutional_belief_claims_default_returns_empty() {
        // The default GoalBeliefView implementation returns empty vec.
        // TestBeliefView uses the macro-generated impl which delegates to RuntimeBeliefView,
        // and RuntimeBeliefView's default returns empty.
        let view = TestBeliefView::default();
        let result = worldwake_sim::GoalBeliefView::institutional_belief_claims(
            &view,
            entity(1),
            worldwake_core::InstitutionalBeliefKey::OfficeHolderOf { office: entity(99) },
        );
        assert!(
            result.is_empty(),
            "default institutional_belief_claims() should return empty, got {result:?}",
        );
    }

    #[test]
    fn social_candidate_produces_evidence_trace() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.insert(speaker, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(subject, remote_place);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 5,
                max_relay_chain_len: 3,
                ..TellProfile::default()
            },
        );
        view.beliefs
            .insert(speaker, vec![known_entity(subject, place)]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(10),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        let has_listener = trace
            .contributors
            .iter()
            .any(|c| c.kind == super::CandidateEvidenceKind::Listener && c.entity == listener);
        let has_subject = trace
            .contributors
            .iter()
            .any(|c| c.kind == super::CandidateEvidenceKind::TellSubject && c.entity == subject);
        assert!(
            has_listener,
            "evidence trace should contain Listener contributor, got {:?}",
            trace.contributors,
        );
        assert!(
            has_subject,
            "evidence trace should contain TellSubject contributor, got {:?}",
            trace.contributors,
        );
    }

    #[test]
    fn social_candidate_knowledge_path_records_subject_belief() {
        let speaker = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([speaker, listener, subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(subject, EntityKind::Agent);
        view.effective_places.insert(speaker, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(subject, remote_place);
        view.entities_at.insert(place, vec![speaker, listener]);
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 5,
                max_relay_chain_len: 3,
                ..TellProfile::default()
            },
        );
        view.beliefs.insert(
            speaker,
            vec![(
                subject,
                BelievedEntityState {
                    last_known_place: Some(place),
                    ..believed_state(
                        7,
                        PerceptionSource::Report {
                            from: listener,
                            chain_len: 1,
                        },
                    )
                },
            )],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            speaker,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(10),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace.knowledge_path.entity_beliefs.contains(
                &crate::knowledge_path::BeliefProvenance {
                    subject,
                    aspect: BeliefAspect::LocationAt { place },
                    source: PerceptionSource::Report {
                        from: listener,
                        chain_len: 1,
                    },
                    observed_tick: Tick(7),
                }
            ),
            "knowledge_path.entity_beliefs should contain belief about subject, got {:?}",
            trace.knowledge_path.entity_beliefs,
        );
    }

    #[test]
    fn claim_office_candidate_produces_evidence_trace() {
        let agent = entity(1);
        let office = entity(2);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.entities_at.insert(town, vec![agent]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![faction]);
        view.beliefs.insert(agent, vec![known_entity(office, town)]);

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(10),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::ClaimOffice { office });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        let has_office = trace.contributors.iter().any(|c| {
            c.kind == super::CandidateEvidenceKind::OfficeParticipant && c.entity == office
        });
        let has_agent = trace.contributors.iter().any(|c| {
            c.kind == super::CandidateEvidenceKind::OfficeParticipant && c.entity == agent
        });
        assert!(
            has_office,
            "evidence trace should contain OfficeParticipant for office, got {:?}",
            trace.contributors,
        );
        assert!(
            has_agent,
            "evidence trace should contain OfficeParticipant for agent, got {:?}",
            trace.contributors,
        );
    }

    #[test]
    fn claim_office_knowledge_path_records_institutional_provenance() {
        let agent = entity(1);
        let office = entity(2);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.entities_at.insert(town, vec![agent]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![faction]);
        view.beliefs.insert(agent, vec![known_entity(office, town)]);
        // Configure institutional belief claims
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: Tick(5),
        };
        view.institutional_claims.insert(
            (agent, InstitutionalBeliefKey::OfficeHolderOf { office }),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(6),
                learned_at: Some(town),
            }],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(10),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::ClaimOffice { office });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace
                .knowledge_path
                .institutional_beliefs
                .contains(&InstitutionalBeliefProvenance {
                    claim,
                    source: InstitutionalKnowledgeSource::WitnessedEvent,
                    learned_tick: Tick(6),
                    learned_at: Some(town),
                }),
            "knowledge_path.institutional_beliefs should contain office holder provenance, got {:?}",
            trace.knowledge_path.institutional_beliefs,
        );
    }

    #[test]
    fn support_candidate_knowledge_path_records_institutional_provenance() {
        let agent = entity(1);
        let office = entity(2);
        let candidate = entity(3);
        let town = entity(10);
        let faction = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, candidate]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(candidate, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, town);
        view.effective_places.insert(candidate, town);
        view.entities_at.insert(town, vec![agent, candidate]);
        view.office_data
            .insert(office, vacant_office("Ruler", town, faction));
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.factions_by_member.insert(agent, vec![faction]);
        view.factions_by_member.insert(candidate, vec![faction]);
        view.loyalties.insert((agent, candidate), pm(650));
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );
        // Configure institutional belief claims for support declaration
        let claim = InstitutionalClaim::SupportDeclaration {
            office,
            supporter: agent,
            candidate: None,
            effective_tick: Tick(4),
        };
        view.institutional_claims.insert(
            (
                agent,
                InstitutionalBeliefKey::SupportFor {
                    supporter: agent,
                    office,
                },
            ),
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::SelfDeclaration,
                learned_tick: Tick(4),
                learned_at: Some(town),
            }],
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(10),
            6,
            true,
        );

        let key = GoalKey::from(GoalKind::SupportCandidateForOffice { office, candidate });
        let trace = evidence_trace_for_goal(&result.diagnostics, key);

        assert!(
            trace
                .knowledge_path
                .institutional_beliefs
                .contains(&InstitutionalBeliefProvenance {
                    claim,
                    source: InstitutionalKnowledgeSource::SelfDeclaration,
                    learned_tick: Tick(4),
                    learned_at: Some(town),
                }),
            "knowledge_path.institutional_beliefs should contain support declaration provenance, got {:?}",
            trace.knowledge_path.institutional_beliefs,
        );
    }

    // ── Expectation-violation candidate generation tests ──

    fn default_violation_profile() -> worldwake_core::ViolationDispositionProfile {
        worldwake_core::ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(500),
            ownership_motive_bonus: pm(200),
        }
    }

    fn belief_at_place(place: EntityId, tick: Tick) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            observed_tick: tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn belief_resource_at_place(
        place: EntityId,
        commodity: CommodityKind,
        qty: u32,
        tick: Tick,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: Some(ResourceSource {
                commodity,
                available_quantity: Quantity(qty),
                max_quantity: Quantity(qty),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            }),
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            observed_tick: tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    // Test 1: EntityMissing violation detected, InvestigateViolation candidate emitted
    #[test]
    fn violation_entity_missing_emits_investigate_candidate() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes entity 2 is at place 10 (stale belief).
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        // Entity 2 is NOT in entities_at(place) — it's gone.
        view.entities_at.insert(place, vec![agent]);
        // Entity 2 still has an effective place (somewhere else, not in transit).
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let violation_id = result.pending_violations[0].id;
        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id,
            place,
        });
        assert!(
            result.candidates.iter().any(|c| c.key == goal_key),
            "Expected InvestigateViolation candidate, got: {:?}",
            result.candidates.iter().map(|c| c.key).collect::<Vec<_>>()
        );
        assert!(
            !result.pending_violations.is_empty(),
            "Expected pending violation record"
        );
    }

    // Test 2: SupplyDepleted violation detected, InvestigateViolation candidate emitted
    #[test]
    fn violation_supply_depleted_emits_investigate_candidate() {
        let agent = entity(1);
        let place = entity(10);
        let source_entity = entity(3);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes source has apples (qty 5) at place.
        view.beliefs.insert(
            agent,
            vec![(
                source_entity,
                belief_resource_at_place(place, CommodityKind::Apple, 5, Tick(1)),
            )],
        );
        // Source is present but commodity quantity is now 0.
        view.entities_at.insert(place, vec![agent, source_entity]);
        view.effective_places.insert(source_entity, place);
        view.commodity_quantities
            .insert((source_entity, CommodityKind::Apple), Quantity(0));

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let violation_id = result.pending_violations[0].id;
        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id,
            place,
        });
        assert!(
            result.candidates.iter().any(|c| c.key == goal_key),
            "Expected InvestigateViolation candidate for depleted supply"
        );
    }

    #[test]
    fn same_place_distinct_violations_emit_distinct_investigate_goals() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);
        let source_entity = entity(3);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![
                (missing_entity, belief_at_place(place, Tick(1))),
                (
                    source_entity,
                    belief_resource_at_place(place, CommodityKind::Apple, 5, Tick(2)),
                ),
            ],
        );
        view.entities_at.insert(place, vec![agent, source_entity]);
        view.effective_places.insert(missing_entity, entity(20));
        view.effective_places.insert(source_entity, place);
        view.commodity_quantities
            .insert((source_entity, CommodityKind::Apple), Quantity(0));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert_eq!(result.pending_violations.len(), 2);
        assert_eq!(result.candidates.len(), 2);
        assert_ne!(
            result.pending_violations[0].id,
            result.pending_violations[1].id
        );
        assert!(result.candidates.iter().any(|candidate| {
            candidate.key
                == GoalKey::from(GoalKind::InvestigateViolation {
                    violation_id: result.pending_violations[0].id,
                    place,
                })
        }));
        assert!(result.candidates.iter().any(|candidate| {
            candidate.key
                == GoalKey::from(GoalKind::InvestigateViolation {
                    violation_id: result.pending_violations[1].id,
                    place,
                })
        }));
    }

    // Test 3: EntityDead records in ViolationMemory but does NOT emit InvestigateViolation
    #[test]
    fn violation_entity_dead_records_only_no_goal() {
        let agent = entity(1);
        let place = entity(10);
        let dead_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes entity 2 is alive at place.
        view.beliefs
            .insert(agent, vec![(dead_entity, belief_at_place(place, Tick(1)))]);
        // Entity 2 IS present but dead.
        view.entities_at.insert(place, vec![agent, dead_entity]);
        view.effective_places.insert(dead_entity, place);
        view.dead.insert(dead_entity);

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(0),
            place,
        });
        assert!(
            !result.candidates.iter().any(|c| c.key == goal_key),
            "EntityDead should NOT emit InvestigateViolation"
        );
        // But should still produce a pending violation record.
        assert!(
            result.pending_violations.iter().any(|pv| matches!(
                &pv.kind,
                worldwake_core::ViolationKind::EntityDead { entity } if *entity == dead_entity
            )),
            "EntityDead should produce a pending violation record"
        );
    }

    // Test 4: Already-recorded unresolved violation re-emits candidate without a new pending record
    #[test]
    fn unresolved_recorded_violation_reemits_candidate_without_new_pending_record() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let mut vm = ViolationMemory::default();
        // Pre-record the violation so it's already known.
        vm.record(
            worldwake_core::ViolationKind::EntityMissing {
                entity: missing_entity,
                expected_place: place,
            },
            Tick(3),
            50,
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(0),
            place,
        });
        assert!(
            result.candidates.iter().any(|c| c.key == goal_key),
            "Unresolved recorded violation should remain candidate-eligible"
        );
        assert!(
            result.pending_violations.is_empty(),
            "Already-recorded violation should not produce pending record"
        );
    }

    #[test]
    fn resolved_recorded_violation_does_not_emit_candidate() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let mut vm = ViolationMemory::default();
        let id = vm.record(
            worldwake_core::ViolationKind::EntityMissing {
                entity: missing_entity,
                expected_place: place,
            },
            Tick(3),
            50,
        );
        assert!(vm.resolve_id(id, Tick(4), 50));

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: id,
            place,
        });
        assert!(
            !result.candidates.iter().any(|c| c.key == goal_key),
            "Resolved recorded violation should not emit a fresh investigate candidate"
        );
        assert!(
            result.pending_violations.is_empty(),
            "Resolved recorded violation should not be rediscovered while unexpired"
        );
    }

    #[test]
    fn suspected_theft_record_does_not_emit_generic_investigate_goal() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let mut vm = ViolationMemory::default();
        let id = vm.record(
            worldwake_core::ViolationKind::SuspectedTheft {
                theft: worldwake_core::TheftFacts {
                    missing_entity,
                    expected_place: place,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                },
                suspect: None,
            },
            Tick(3),
            50,
        );

        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: id,
            place,
        });
        assert!(
            !result.candidates.iter().any(|c| c.key == goal_key),
            "SuspectedTheft should not re-enter the generic investigate goal pipeline"
        );
        assert!(
            !result.pending_violations.iter().any(|record| matches!(
                record.kind,
                worldwake_core::ViolationKind::SuspectedTheft { .. }
            )),
            "Candidate generation should not synthesize new SuspectedTheft pending records"
        );
    }

    // Test 5: Blocked investigation goal is skipped
    #[test]
    fn violation_blocked_investigation_is_skipped() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        // Block the investigation goal.
        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: GoalKey::from(GoalKind::InvestigateViolation {
                    violation_id: worldwake_core::ViolationId(0),
                    place,
                }),
                place: None,
                target: None,
                action_def: None,
            },
            blocking_fact: BlockingFact::Unknown,
            diagnostic_context: None,
            observed_tick: Tick(3),
            expires_tick: Tick(100),
        });

        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(0),
            place,
        });
        assert!(
            !result.candidates.iter().any(|c| c.key == goal_key),
            "Blocked investigation should not emit candidate"
        );
        // But the violation should still be recorded.
        assert!(
            !result.pending_violations.is_empty(),
            "Blocked investigation should still produce pending violation record"
        );
    }

    // Test 6: Agent without ViolationDispositionProfile emits no violation candidates
    #[test]
    fn violation_no_profile_emits_nothing() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        // No violation_disposition_profile set.
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            result.pending_violations.is_empty(),
            "No profile should produce no violations"
        );
    }

    // Test 7: Agent in transit (no current place) emits no violation candidates
    #[test]
    fn violation_agent_in_transit_emits_nothing() {
        let agent = entity(1);
        let missing_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        // Agent has NO effective place (in transit).
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(entity(10), Tick(1)))],
        );

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            result.pending_violations.is_empty(),
            "Agent in transit should produce no violations"
        );
    }

    // Test 8: Self-entity excluded from violation checks
    #[test]
    fn violation_self_excluded() {
        let agent = entity(1);
        let place = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes itself was at place (stale self-belief).
        view.beliefs
            .insert(agent, vec![(agent, belief_at_place(place, Tick(1)))]);
        // Agent IS at place but entities_at doesn't include self in observed set
        // (simulating the edge case).
        view.entities_at.insert(place, vec![]);

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            result.pending_violations.is_empty(),
            "Self-entity should never trigger violation"
        );
    }

    // Test 9: Entity with no prior belief at current place does not trigger violation
    #[test]
    fn violation_no_prior_belief_at_place_no_violation() {
        let agent = entity(1);
        let place = entity(10);
        let other_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes entity 2 is at a DIFFERENT place (entity(20)), not at place 10.
        view.beliefs.insert(
            agent,
            vec![(other_entity, belief_at_place(entity(20), Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(other_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            result.pending_violations.is_empty(),
            "No belief about entity at current place should produce no violation"
        );
    }

    // Test 10: EvidenceTrace with KnowledgePath is populated for violation candidate
    #[test]
    fn violation_candidate_has_knowledge_path() {
        let agent = entity(1);
        let place = entity(10);
        let missing_entity = entity(2);

        let mut view = TestBeliefView {
            current_tick: Tick(5),
            ..TestBeliefView::default()
        };
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        view.beliefs.insert(
            agent,
            vec![(missing_entity, belief_at_place(place, Tick(1)))],
        );
        view.entities_at.insert(place, vec![agent]);
        view.effective_places.insert(missing_entity, entity(20));

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            true, // enable tracing
        );

        let violation_id = result.pending_violations[0].id;
        let goal_key = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id,
            place,
        });
        let trace = evidence_trace_for_goal(&result.diagnostics, goal_key);

        assert!(
            !trace.knowledge_path.entity_beliefs.is_empty(),
            "Knowledge path should contain entity belief provenance"
        );
        let belief_prov = &trace.knowledge_path.entity_beliefs[0];
        assert_eq!(belief_prov.subject, missing_entity);
        assert_eq!(belief_prov.observed_tick, Tick(1));
        assert!(
            matches!(belief_prov.aspect, BeliefAspect::LocationAt { place: p } if p == place),
            "Aspect should be LocationAt for EntityMissing violation"
        );
    }

    // Test 11: In-transit entity excluded from violation detection
    // ── Remote pursuit candidate generation tests ──────────────────────

    /// Helper: set up a remote pursuit scenario and return generated candidates.
    fn remote_raid_setup(
        min_confidence: u16,
        max_travel: u32,
        belief_staleness: u64,
        route_hops: usize,
    ) -> (Vec<crate::GroundedGoal>, EntityId) {
        let agent = entity(1);
        let target = entity(2);
        let faction = entity(30);
        let agent_place = entity(10);
        let remote_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        // Target is NOT at agent_place (remote).
        view.effective_places.insert(target, remote_place);
        view.entities_at
            .insert(agent_place, vec![agent]);
        view.entities_at.insert(remote_place, vec![target]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        // Agent believes target is at remote_place.
        let observed_tick = Tick(100u64.saturating_sub(belief_staleness));
        view.beliefs.insert(
            agent,
            vec![(target, belief_at_place(remote_place, observed_tick))],
        );
        // Pursuit profile.
        view.pursuit_profiles.insert(
            agent,
            worldwake_core::PursuitProfile {
                min_location_confidence: Permille::new(min_confidence).unwrap(),
                max_pursuit_travel_ticks: NonZeroU32::new(max_travel).unwrap(),
            },
        );
        // Build adjacency chain: agent_place -> intermediate -> ... -> remote_place
        // Each hop costs 1 tick (TestBeliefView default).
        let mut places = vec![agent_place];
        for i in 1..route_hops {
            places.push(entity(100 + i as u32));
        }
        places.push(remote_place);
        for w in places.windows(2) {
            view.adjacent_places
                .entry(w[0])
                .or_default()
                .push(w[1]);
            view.adjacent_places
                .entry(w[1])
                .or_default()
                .push(w[0]);
        }

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(100),
        );
        (candidates, target)
    }

    #[test]
    fn remote_raid_target_emitted_when_pursuit_conditions_met() {
        // min_confidence=500, max_travel=5, staleness=0 (confidence=950), hops=2
        let (candidates, target) = remote_raid_setup(500, 5, 0, 2);
        assert!(
            contains_goal(&candidates, GoalKind::RaidTarget { target }),
            "Remote RaidTarget should be emitted when confidence and route cost pass"
        );
    }

    #[test]
    fn remote_raid_target_omitted_when_confidence_too_low() {
        // min_confidence=960, staleness=5 (confidence=950-60=890 < 960), hops=1
        let (candidates, target) = remote_raid_setup(960, 10, 5, 1);
        assert!(
            !contains_goal(&candidates, GoalKind::RaidTarget { target }),
            "Remote RaidTarget should NOT be emitted when confidence < min_location_confidence"
        );
    }

    #[test]
    fn remote_raid_target_omitted_when_route_too_long() {
        // min_confidence=500, max_travel=1, staleness=0, hops=3 (route cost = 3 > 1)
        let (candidates, target) = remote_raid_setup(500, 1, 0, 3);
        assert!(
            !contains_goal(&candidates, GoalKind::RaidTarget { target }),
            "Remote RaidTarget should NOT be emitted when route cost > max_pursuit_travel_ticks"
        );
    }

    #[test]
    fn remote_raid_target_omitted_when_blocked() {
        let agent = entity(1);
        let target = entity(2);
        let faction = entity(30);
        let agent_place = entity(10);
        let remote_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        view.effective_places.insert(target, remote_place);
        view.entities_at
            .insert(agent_place, vec![agent]);
        view.entities_at.insert(remote_place, vec![target]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        view.beliefs.insert(
            agent,
            vec![(target, belief_at_place(remote_place, Tick(100)))],
        );
        view.pursuit_profiles.insert(
            agent,
            worldwake_core::PursuitProfile {
                min_location_confidence: Permille::new(500).unwrap(),
                max_pursuit_travel_ticks: NonZeroU32::new(5).unwrap(),
            },
        );
        view.adjacent_places
            .insert(agent_place, vec![remote_place]);
        view.adjacent_places
            .insert(remote_place, vec![agent_place]);

        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: GoalKey::from(GoalKind::RaidTarget { target }),
                place: Some(remote_place),
                target: Some(target),
                action_def: None,
            },
            blocking_fact: BlockingFact::TargetGone,
            diagnostic_context: None,
            observed_tick: Tick(99),
            expires_tick: Tick(200),
        });

        let candidates = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &ViolationMemory::default(),
            &RecipeRegistry::new(),
            Tick(100),
            6,
            false,
        )
        .candidates;

        assert!(
            !contains_goal(&candidates, GoalKind::RaidTarget { target }),
            "Remote RaidTarget should NOT be emitted when target/place is blocked"
        );
    }

    #[test]
    fn remote_raid_target_omitted_when_target_place_unknown() {
        let agent = entity(1);
        let target = entity(2);
        let faction = entity(30);
        let agent_place = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        // Target has no effective place and no belief about place.
        view.entities_at
            .insert(agent_place, vec![agent]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_factions.insert(faction);
        // Belief with no place.
        let mut state = belief_at_place(agent_place, Tick(100));
        state.last_known_place = None;
        view.beliefs.insert(agent, vec![(target, state)]);
        view.pursuit_profiles.insert(
            agent,
            worldwake_core::PursuitProfile {
                min_location_confidence: Permille::new(500).unwrap(),
                max_pursuit_travel_ticks: NonZeroU32::new(5).unwrap(),
            },
        );

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(100),
        );

        assert!(
            !contains_goal(&candidates, GoalKind::RaidTarget { target }),
            "Remote RaidTarget should NOT be emitted when target place is unknown"
        );
    }

    #[test]
    fn remote_engage_hostile_emitted_when_pursuit_conditions_met() {
        let agent = entity(1);
        let target = entity(2);
        let agent_place = entity(10);
        let remote_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        view.effective_places.insert(target, remote_place);
        view.entities_at
            .insert(agent_place, vec![agent]);
        view.entities_at.insert(remote_place, vec![target]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![target]);
        view.beliefs.insert(
            agent,
            vec![(target, belief_at_place(remote_place, Tick(100)))],
        );
        view.pursuit_profiles.insert(
            agent,
            worldwake_core::PursuitProfile {
                min_location_confidence: Permille::new(500).unwrap(),
                max_pursuit_travel_ticks: NonZeroU32::new(5).unwrap(),
            },
        );
        view.adjacent_places
            .insert(agent_place, vec![remote_place]);
        view.adjacent_places
            .insert(remote_place, vec![agent_place]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(100),
        );

        assert!(
            contains_goal(&candidates, GoalKind::EngageHostile { target }),
            "Remote EngageHostile should be emitted when pursuit conditions met"
        );
    }

    #[test]
    fn remote_engage_hostile_omitted_when_confidence_too_low() {
        let agent = entity(1);
        let target = entity(2);
        let agent_place = entity(10);
        let remote_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        view.effective_places.insert(target, remote_place);
        view.entities_at
            .insert(agent_place, vec![agent]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.hostiles.insert(agent, vec![target]);
        // Observed at tick 10, current=100 → staleness=90 → penalty=90*12=1080 → confidence=0
        view.beliefs.insert(
            agent,
            vec![(target, belief_at_place(remote_place, Tick(10)))],
        );
        view.pursuit_profiles.insert(
            agent,
            worldwake_core::PursuitProfile {
                min_location_confidence: Permille::new(500).unwrap(),
                max_pursuit_travel_ticks: NonZeroU32::new(5).unwrap(),
            },
        );
        view.adjacent_places
            .insert(agent_place, vec![remote_place]);
        view.adjacent_places
            .insert(remote_place, vec![agent_place]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(100),
        );

        assert!(
            !contains_goal(&candidates, GoalKind::EngageHostile { target }),
            "Remote EngageHostile should NOT be emitted when confidence too low"
        );
    }

    #[test]
    fn violation_in_transit_entity_excluded() {
        let agent = entity(1);
        let place = entity(10);
        let traveling_entity = entity(2);

        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.violation_disposition_profiles
            .insert(agent, default_violation_profile());
        // Agent believes entity 2 was at place.
        view.beliefs.insert(
            agent,
            vec![(traveling_entity, belief_at_place(place, Tick(1)))],
        );
        // Entity 2 is NOT at place (traveling).
        view.entities_at.insert(place, vec![agent]);
        // Entity 2 has NO effective place (it's on a travel edge).
        // (effective_places does not contain traveling_entity)

        let blocked = BlockedIntentMemory::default();
        let vm = ViolationMemory::default();
        let result = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &blocked,
            &vm,
            &RecipeRegistry::new(),
            Tick(5),
            6,
            false,
        );

        assert!(
            result.pending_violations.is_empty(),
            "In-transit entity (no effective place) should not trigger EntityMissing"
        );
    }
}
