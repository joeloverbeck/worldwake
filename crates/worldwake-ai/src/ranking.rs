use crate::{
    DecisionContext, GoalKindPlannerExt, GoalPolicyOutcome, GoalPriorityClass, GroundedGoal,
    RankedDriveGoalProvenance, RankedDriveKind, RankedDriveMotiveInput, RankedGoal,
    RankedGoalProvenance, RankedGoalProvenanceFamily, RankedPriorityAdjustment, assess_danger,
    classify_band,
    decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    derive_danger_pressure, derive_pain_pressure,
    enterprise::{market_signal_for_place, opportunity_signal},
    evaluate_suppression,
    pressure::is_bandit_raid_deterred_by_wounds,
    route_threat::threat_warning_signal_for_place,
    theft::assess_theft_deterrence,
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};
use worldwake_core::{
    ActionDomain, BelievedEntityState, BountyTarget, CommodityKind, CommodityPurpose,
    CommunicationClass, DriveThresholds, EntityId, GoalKey, GoalKind, HomeostaticNeeds,
    InstitutionalBeliefRead, InstitutionalClaim, InstitutionalKnowledgeSource, NoticeTopic,
    OpportunityAnchor, OpportunityKey, PerceptionSource, Permille, Quantity, RightKind, SourceKey,
    TellTopic, ThresholdBand, Tick, UtilityProfile, ViolationKind, belief_confidence,
    failure_ratio_permille,
};
use worldwake_sim::{CommodityOpportunityBreakdown, GoalBeliefView, commodity_opportunity_score};

/// Outcome of the ranking pipeline, preserving information about filtered candidates.
#[derive(Clone, Debug)]
pub struct RankingOutcome {
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoal>,
    /// Goals that were suppressed by situational conditions (danger/self-care pressure).
    pub suppressed: Vec<GoalKey>,
    /// Goals that passed suppression but had zero motive score.
    pub zero_motive: Vec<GoalKey>,
}

impl RankingOutcome {
    /// Consume the outcome, returning only the ranked goals.
    #[must_use]
    pub fn into_ranked(self) -> Vec<RankedGoal> {
        self.ranked
    }
}

/// Build a `DecisionContext` from a belief view by computing the two pressure
/// classifications (self-care and danger) that suppression and priority logic need.
#[must_use]
pub fn build_decision_context(view: &dyn GoalBeliefView, agent: EntityId) -> DecisionContext {
    let needs = view.homeostatic_needs(agent);
    let thresholds = view.drive_thresholds(agent);
    let danger_pressure = derive_danger_pressure(view, agent);

    let danger_class = thresholds.map_or(GoalPriorityClass::Background, |t| {
        classify_band(danger_pressure, &t.danger)
    });

    let max_self_care_class = match (needs, thresholds) {
        (Some(needs), Some(t)) => [
            classify_band(needs.hunger, &t.hunger),
            classify_band(needs.thirst, &t.thirst),
            classify_band(needs.fatigue, &t.fatigue),
            classify_band(needs.bladder, &t.bladder),
            classify_band(needs.dirtiness, &t.dirtiness),
        ]
        .into_iter()
        .max()
        .unwrap_or(GoalPriorityClass::Background),
        _ => GoalPriorityClass::Background,
    };

    DecisionContext {
        max_self_care_class,
        danger_class,
    }
}

#[must_use]
pub fn rank_candidates(
    candidates: &[GroundedGoal],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    decision_context: &DecisionContext,
) -> RankingOutcome {
    let context = RankingContext::new(view, agent, current_tick, utility, *decision_context);

    let mut suppressed = Vec::new();
    let mut zero_motive = Vec::new();

    let mut ranked = Vec::new();
    for candidate in candidates {
        if !matches!(
            evaluate_suppression(&candidate.key.kind, &context.decision_context),
            GoalPolicyOutcome::Available
        ) {
            suppressed.push(candidate.key);
            continue;
        }
        let provenance = goal_ranking_provenance(candidate, &context);
        let priority_class = ranked_priority_class(candidate, &context, provenance.as_ref());
        let motive_score = ranked_motive_score(candidate, &context, provenance.as_ref());
        let source_reliability_discount =
            apply_source_reliability_discount(candidate, &context, motive_score);
        let post_source_reliability_motive = source_reliability_discount
            .as_ref()
            .map_or(motive_score, |discount| discount.post_discount_motive);
        let competition_discount =
            apply_competition_discount(candidate, &context, post_source_reliability_motive);
        let scored = RankedGoal {
            grounded: candidate.clone(),
            priority_class,
            motive_score: competition_discount
                .as_ref()
                .map_or(post_source_reliability_motive, |discount| {
                    discount.post_discount_motive
                }),
            provenance,
            source_reliability_discount,
            competition_discount,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
        };
        if scored.motive_score == 0 {
            zero_motive.push(candidate.key);
        } else {
            ranked.push(scored);
        }
    }

    ranked.sort_unstable_by(compare_ranked_goals);
    RankingOutcome {
        ranked,
        suppressed,
        zero_motive,
    }
}

fn ranked_priority_class(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    provenance: Option<&RankedGoalProvenance>,
) -> GoalPriorityClass {
    provenance.cloned().map_or_else(
        || priority_class(candidate, context),
        |provenance| match provenance {
            RankedGoalProvenance::Danger(_) => context.decision_context.danger_class,
            RankedGoalProvenance::Drive(provenance) => provenance.final_priority_class,
        },
    )
}

fn ranked_motive_score(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    provenance: Option<&RankedGoalProvenance>,
) -> u32 {
    provenance.cloned().map_or_else(
        || motive_score(candidate, context),
        |provenance| match provenance {
            RankedGoalProvenance::Danger(_) => {
                score_product(context.utility.danger_weight, context.danger_pressure)
            }
            RankedGoalProvenance::Drive(provenance) => provenance
                .motive_inputs
                .iter()
                .map(|input| input.score)
                .max()
                .unwrap_or(0),
        },
    )
}

fn apply_competition_discount(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    motive_score: u32,
) -> Option<CompetitionDiscount> {
    if motive_score == 0 {
        return None;
    }

    let (domain, place) = competition_discount_scope(candidate)?;
    let observed_competitors = context.view.agents_active_at(place, domain, None);
    if observed_competitors.is_empty() {
        return None;
    }

    let observed_count = u32::try_from(observed_competitors.len()).unwrap_or(u32::MAX);
    let effective_count = observed_count.min(3);
    let awareness = u32::from(context.utility.activity_awareness_weight.value());
    let factor = 1000u32.saturating_sub(awareness.saturating_mul(effective_count));
    let post_discount_motive = motive_score.saturating_mul(factor) / 1000;

    Some(CompetitionDiscount {
        observed_competitors,
        domain,
        effective_discount: Permille::new((1000 - factor) as u16).unwrap(),
        pre_discount_motive: motive_score,
        post_discount_motive: post_discount_motive.max(1),
    })
}

fn apply_source_reliability_discount(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    motive_score: u32,
) -> Option<SourceReliabilityDiscount> {
    if motive_score == 0 {
        return None;
    }

    let (source_entity, commodity) = source_reliability_discount_scope(candidate)?;
    let source_reliability = context.view.source_reliability(context.agent)?;
    let profile = context.view.preference_profile(context.agent)?;
    let record = source_reliability.sources.get(&SourceKey {
        entity: source_entity,
        commodity,
    })?;
    let failure_ratio = failure_ratio_permille(record);
    if failure_ratio == 0 {
        return None;
    }

    let trust_weight = u32::from(profile.source_trust_weight.value());
    let effective_discount = trust_weight.saturating_mul(failure_ratio) / 1000;
    let post_discount_motive =
        (motive_score.saturating_mul(1000u32.saturating_sub(effective_discount)) / 1000).max(1);

    Some(SourceReliabilityDiscount {
        source_entity,
        commodity,
        failure_ratio_permille: failure_ratio,
        pre_discount_motive: motive_score,
        post_discount_motive,
    })
}

fn source_reliability_discount_scope(
    candidate: &GroundedGoal,
) -> Option<(EntityId, CommodityKind)> {
    let mut source_entities = candidate.evidence_entities.iter().copied();
    let source_entity = source_entities.next()?;
    if source_entities.next().is_some() {
        return None;
    }

    match candidate.key.kind {
        GoalKind::AcquireCommodity { commodity, .. } | GoalKind::RestockCommodity { commodity } => {
            Some((source_entity, commodity))
        }
        _ => None,
    }
}

fn competition_discount_scope(candidate: &GroundedGoal) -> Option<(ActionDomain, EntityId)> {
    let place = match candidate.anchor {
        OpportunityAnchor::Place(place) => place,
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => return None,
    };

    match candidate.key.kind {
        GoalKind::ProduceCommodity { .. } | GoalKind::RestockCommodity { .. } => {
            Some((ActionDomain::Production, place))
        }
        _ => None,
    }
}

fn goal_ranking_provenance(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
) -> Option<RankedGoalProvenance> {
    match candidate.key.kind.ranked_goal_provenance_family() {
        Some(RankedGoalProvenanceFamily::Drive) => {
            drive_goal_ranking_provenance(&candidate.key.kind, context)
        }
        Some(RankedGoalProvenanceFamily::Danger) => Some(RankedGoalProvenance::Danger(
            context.danger_assessment.clone(),
        )),
        None => None,
    }
}

fn drive_goal_ranking_provenance(
    goal_kind: &GoalKind,
    context: &RankingContext<'_>,
) -> Option<RankedGoalProvenance> {
    match goal_kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => self_consume_provenance(*commodity, context).map(RankedGoalProvenance::Drive),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        }
        | GoalKind::ProduceCommodity { recipe_id } => {
            best_recipe_output_assessment(*recipe_id, context)
                .and_then(|assessment| assessment.provenance)
                .map(RankedGoalProvenance::Drive)
        }
        GoalKind::Sleep => drive_goal_provenance(
            context,
            RankedDriveKind::Fatigue,
            |needs| needs.fatigue,
            |thresholds| thresholds.fatigue,
            |utility| utility.fatigue_weight,
            true,
        )
        .map(RankedGoalProvenance::Drive),
        GoalKind::Relieve => drive_goal_provenance(
            context,
            RankedDriveKind::Bladder,
            |needs| needs.bladder,
            |thresholds| thresholds.bladder,
            |utility| utility.bladder_weight,
            false,
        )
        .map(RankedGoalProvenance::Drive),
        GoalKind::Wash => drive_goal_provenance(
            context,
            RankedDriveKind::Dirtiness,
            |needs| needs.dirtiness,
            |thresholds| thresholds.dirtiness,
            |utility| utility.dirtiness_weight,
            false,
        )
        .map(RankedGoalProvenance::Drive),
        _ => None,
    }
}

struct RankingContext<'a> {
    view: &'a dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &'a UtilityProfile,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    has_clotted_wounds: bool,
    danger_assessment: crate::DangerAssessment,
    danger_pressure: Permille,
    decision_context: DecisionContext,
    holdings: BTreeMap<CommodityKind, u32>,
    local_alternatives: BTreeMap<CommodityKind, u32>,
}

impl<'a> RankingContext<'a> {
    fn new(
        view: &'a dyn GoalBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &'a UtilityProfile,
        decision_context: DecisionContext,
    ) -> Self {
        let danger_assessment = assess_danger(view, agent);
        Self {
            view,
            agent,
            current_tick,
            utility,
            needs: view.homeostatic_needs(agent),
            thresholds: view.drive_thresholds(agent),
            has_clotted_wounds: has_clotted_wounds(view, agent),
            danger_pressure: danger_assessment.pressure,
            danger_assessment,
            decision_context,
            holdings: holdings_from_view(view, agent),
            local_alternatives: local_alternatives_from_view(view, agent),
        }
    }
}

#[derive(Copy, Clone)]
struct DriveFactor {
    drive: RankedDriveKind,
    pressure: Permille,
    weight: Permille,
    band: ThresholdBand,
    recovery_relevant: bool,
    relief_per_unit: Permille,
}

fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent)
        .into_iter()
        .any(|wound| wound.severity.value() > 0 && wound.bleed_rate_per_tick.value() == 0)
}

fn priority_class(candidate: &GroundedGoal, context: &RankingContext<'_>) -> GoalPriorityClass {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => self_consume_priority(commodity, context),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        }
        | GoalKind::ProduceCommodity { recipe_id } => {
            best_recipe_output_assessment(recipe_id, context)
                .map_or(GoalPriorityClass::Background, |assessment| {
                    assessment.priority_class
                })
        }
        GoalKind::AcquireCommodity { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::MoveCargo { .. }
        | GoalKind::RaidTarget { .. }
        | GoalKind::RegroupWithFaction { .. }
        | GoalKind::EstablishBanditCamp { .. }
        | GoalKind::FulfillBounty { .. }
        | GoalKind::PostBounty { .. }
        | GoalKind::PostNotice { .. }
        | GoalKind::ClaimOffice { .. }
        | GoalKind::SupportCandidateForOffice { .. } => GoalPriorityClass::Medium,
        GoalKind::Sleep => drive_priority(
            context,
            |needs| needs.fatigue,
            |thresholds| thresholds.fatigue,
            true,
        ),
        GoalKind::Relieve => drive_priority(
            context,
            |needs| needs.bladder,
            |thresholds| thresholds.bladder,
            false,
        ),
        GoalKind::Wash => drive_priority(
            context,
            |needs| needs.dirtiness,
            |thresholds| thresholds.dirtiness,
            false,
        ),
        GoalKind::EngageHostile { .. } | GoalKind::ReduceDanger => {
            context.decision_context.danger_class
        }
        GoalKind::TreatWounds { patient } => {
            let patient_pain = derive_pain_pressure(context.view, patient);
            context
                .thresholds
                .map_or(GoalPriorityClass::Background, |thresholds| {
                    classify_band(patient_pain, &thresholds.pain)
                })
        }
        GoalKind::LootCorpse { .. }
        | GoalKind::BuryCorpse { .. }
        | GoalKind::SearchForMissing { .. }
        | GoalKind::ReportMissing { .. }
        | GoalKind::EscortToSafety { .. }
        | GoalKind::ShareBelief { .. }
        | GoalKind::InvestigateViolation { .. }
        | GoalKind::Patrol { .. }
        | GoalKind::StealItem { .. }
        | GoalKind::Accuse { .. }
        | GoalKind::PunishAccused { .. } => GoalPriorityClass::Low,
    }
}

fn self_consume_priority(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> GoalPriorityClass {
    relevant_self_consume_factors(commodity, context)
        .into_iter()
        .map(|factor| {
            promote_for_clotted_wound_recovery(
                classify_band(factor.pressure, &factor.band),
                context,
                factor.recovery_relevant,
            )
        })
        .max()
        .unwrap_or(GoalPriorityClass::Background)
}

fn drive_priority(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    band: impl Fn(DriveThresholds) -> ThresholdBand,
    recovery_relevant: bool,
) -> GoalPriorityClass {
    let base = match (context.needs, context.thresholds) {
        (Some(needs), Some(thresholds)) => classify_band(pressure(needs), &band(thresholds)),
        _ => GoalPriorityClass::Background,
    };

    promote_for_clotted_wound_recovery(base, context, recovery_relevant)
}

fn drive_goal_provenance(
    context: &RankingContext<'_>,
    drive: RankedDriveKind,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    band: impl Fn(DriveThresholds) -> ThresholdBand,
    weight: impl Fn(&UtilityProfile) -> Permille,
    recovery_relevant: bool,
) -> Option<RankedDriveGoalProvenance> {
    let (Some(needs), Some(thresholds)) = (context.needs, context.thresholds) else {
        return None;
    };
    let pressure = pressure(needs);
    let weight = weight(context.utility);
    let base_priority_class = classify_band(pressure, &band(thresholds));
    Some(drive_provenance_from_inputs(
        context,
        base_priority_class,
        vec![RankedDriveMotiveInput {
            drive,
            pressure,
            weight,
            score: score_product(weight, pressure),
            relief_per_unit: Permille::new_unchecked(1000),
            recovery_relevant,
        }],
    ))
}

fn promote_for_clotted_wound_recovery(
    base: GoalPriorityClass,
    context: &RankingContext<'_>,
    recovery_relevant: bool,
) -> GoalPriorityClass {
    // Keep ranking aligned with combat recovery_conditions_met(): hunger, thirst, and fatigue
    // at High block recovery for clotted wounds until the agent resolves that need.
    if recovery_relevant && context.has_clotted_wounds && base == GoalPriorityClass::High {
        GoalPriorityClass::Critical
    } else {
        base
    }
}

fn drive_provenance_from_inputs(
    context: &RankingContext<'_>,
    base_priority_class: GoalPriorityClass,
    motive_inputs: Vec<RankedDriveMotiveInput>,
) -> RankedDriveGoalProvenance {
    let recovery_relevant = motive_inputs.iter().any(|input| input.recovery_relevant);
    let final_priority_class =
        promote_for_clotted_wound_recovery(base_priority_class, context, recovery_relevant);
    RankedDriveGoalProvenance {
        base_priority_class,
        final_priority_class,
        adjustment: (final_priority_class != base_priority_class)
            .then_some(RankedPriorityAdjustment::ClottedWoundRecoveryPromotion),
        motive_inputs,
    }
}

fn motive_score(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => relevant_self_consume_factors(commodity, context)
            .into_iter()
            .map(|factor| score_product(factor.weight, factor.pressure))
            .max()
            .unwrap_or(0),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        }
        | GoalKind::ProduceCommodity { recipe_id } => {
            best_recipe_output_assessment(recipe_id, context)
                .map_or(0, |assessment| assessment.motive_score)
        }
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::SellCommodity { commodity }
        | GoalKind::RestockCommodity { commodity } => enterprise_score(commodity, context),
        GoalKind::Sleep => drive_score(
            context,
            |needs| needs.fatigue,
            |utility| utility.fatigue_weight,
        ),
        GoalKind::Relieve => drive_score(
            context,
            |needs| needs.bladder,
            |utility| utility.bladder_weight,
        ),
        GoalKind::Wash => drive_score(
            context,
            |needs| needs.dirtiness,
            |utility| utility.dirtiness_weight,
        ),
        GoalKind::EngageHostile { .. } | GoalKind::ReduceDanger => {
            score_product(context.utility.danger_weight, context.danger_pressure)
        }
        GoalKind::RaidTarget { .. } => raid_target_motive(candidate, context),
        GoalKind::FulfillBounty { bounty } => context
            .view
            .known_entity_beliefs(context.agent)
            .into_iter()
            .find_map(|(entity, belief)| (entity == bounty).then_some(belief))
            .and_then(|belief| belief.believed_artifact)
            .and_then(|artifact| artifact.bounty_terms)
            .map_or(0, |terms| {
                score_product(
                    context.utility.enterprise_weight,
                    reward_signal_from_quantity(terms.reward_quantity),
                )
            }),
        GoalKind::PostBounty { posting, terms } => post_bounty_motive(context, posting, terms),
        GoalKind::PostNotice { posting, topic } => post_notice_motive(context, posting, topic),
        GoalKind::ClaimOffice { .. } => u32::from(context.utility.enterprise_weight.value()),
        GoalKind::RegroupWithFaction { .. } => u32::from(context.utility.social_weight.value()),
        GoalKind::EstablishBanditCamp { .. } => {
            score_product(context.utility.social_weight, Permille::new_unchecked(1000))
        }
        GoalKind::TreatWounds { patient } => {
            let patient_pain = derive_pain_pressure(context.view, patient);
            if patient == context.agent {
                score_product(context.utility.pain_weight, patient_pain)
            } else {
                score_product(context.utility.care_weight, patient_pain)
            }
        }
        GoalKind::MoveCargo {
            commodity,
            destination,
        } => {
            let signal =
                market_signal_for_place(context.view, context.agent, commodity, destination);
            score_product(context.utility.enterprise_weight, signal)
        }
        GoalKind::ShareBelief {
            topic,
            communication_class,
            ..
        } => {
            let pressure = social_pressure_for_topic(context, topic);
            let boosted_pressure = match communication_class {
                CommunicationClass::Alarm => {
                    pressure.saturating_add(pressure).saturating_add(pressure)
                }
                CommunicationClass::Testimony | CommunicationClass::Gossip => pressure,
            };
            score_product(context.utility.social_weight, boosted_pressure)
        }
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } => 1,
        GoalKind::SearchForMissing { .. }
        | GoalKind::ReportMissing { .. }
        | GoalKind::EscortToSafety { .. } => 0,
        GoalKind::Patrol { .. } => patrol_motive(context),
        GoalKind::StealItem { .. } => theft_motive(context),
        GoalKind::Accuse { .. } | GoalKind::PunishAccused { .. } => justice_motive(context),
        GoalKind::InvestigateViolation { .. } => investigation_motive(candidate, context),
        GoalKind::SupportCandidateForOffice { candidate, .. } => context
            .view
            .loyalty_to(context.agent, candidate)
            .map_or(0, |loyalty| {
                score_product(context.utility.social_weight, loyalty)
            }),
    }
}

fn social_pressure_for_topic(context: &RankingContext<'_>, topic: TellTopic) -> Permille {
    let policy = context.view.belief_confidence_policy(context.agent);
    match topic {
        TellTopic::EntityBelief { subject } => {
            let belief = context
                .view
                .known_entity_beliefs(context.agent)
                .into_iter()
                .find_map(|(entity, belief)| (entity == subject).then_some(belief));

            belief.map_or(Permille::new_unchecked(0), |belief| {
                belief_pressure_from_state(&belief, context.current_tick, &policy)
            })
        }
        TellTopic::SocialObservation { observation } => belief_pressure_from_source(
            observation.source,
            observation.observed_tick,
            context.current_tick,
            &policy,
        ),
        TellTopic::InstitutionalClaim { claim } => context
            .view
            .known_institutional_beliefs(context.agent)
            .into_iter()
            .filter(|belief| {
                worldwake_core::institutional_claim_same_memory_lane(belief.claim, claim)
            })
            .max_by_key(|belief| {
                (
                    worldwake_core::Tick(match belief.claim {
                        worldwake_core::InstitutionalClaim::OfficeHolder {
                            effective_tick, ..
                        }
                        | worldwake_core::InstitutionalClaim::ForceControl {
                            effective_tick, ..
                        }
                        | worldwake_core::InstitutionalClaim::FactionMembership {
                            effective_tick,
                            ..
                        }
                        | worldwake_core::InstitutionalClaim::FactionRallyPoint {
                            effective_tick,
                            ..
                        }
                        | worldwake_core::InstitutionalClaim::SupportDeclaration {
                            effective_tick,
                            ..
                        }
                        | worldwake_core::InstitutionalClaim::Accusation {
                            effective_tick, ..
                        }
                        | worldwake_core::InstitutionalClaim::Verdict { effective_tick, .. } => {
                            effective_tick.0
                        }
                    }),
                    std::cmp::Reverse(worldwake_core::institutional_knowledge_chain_len(
                        belief.source,
                    )),
                    belief.learned_tick,
                    belief.learned_at,
                )
            })
            .map_or(Permille::new_unchecked(0), |belief| {
                belief_pressure_from_source(
                    match belief.source {
                        worldwake_core::InstitutionalKnowledgeSource::DirectObservation
                        | worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent
                        | worldwake_core::InstitutionalKnowledgeSource::RecordConsultation {
                            ..
                        }
                        | worldwake_core::InstitutionalKnowledgeSource::SelfDeclaration => {
                            worldwake_core::PerceptionSource::DirectObservation
                        }
                        worldwake_core::InstitutionalKnowledgeSource::Report {
                            from,
                            chain_len,
                        } => worldwake_core::PerceptionSource::Report { from, chain_len },
                    },
                    belief.learned_tick,
                    context.current_tick,
                    &policy,
                )
            }),
    }
}

fn theft_motive(context: &RankingContext<'_>) -> u32 {
    assess_theft_deterrence(context.view, context.agent)
        .map_or(0, |deterrence| deterrence.effective_motive)
}

fn justice_motive(context: &RankingContext<'_>) -> u32 {
    context
        .view
        .justice_disposition_profile(context.agent)
        .map_or(0, |profile| {
            u32::from(profile.accusation_motive_weight.value())
        })
}

fn post_bounty_motive(
    context: &RankingContext<'_>,
    posting: worldwake_core::ArtifactPostingContext,
    terms: worldwake_core::BountyTerms,
) -> u32 {
    let (Some(office), BountyTarget::EliminateEntity { target }) =
        (posting.issuing_authority, terms.target)
    else {
        return 0;
    };

    let Some(office_data) = context.view.office_data(office) else {
        return 0;
    };
    if office_data.seat != posting.posting_place {
        return 0;
    }
    if !matches!(
        context.view.believed_office_holder(office),
        InstitutionalBeliefRead::Certain(Some(holder)) if holder == context.agent
    ) {
        return 0;
    }
    if !context
        .view
        .believed_rights(context.agent, target)
        .into_iter()
        .any(|right| right.kind == RightKind::JurisdictionalAuthority && right.via == Some(office))
    {
        return 0;
    }

    let accusation_matches = context
        .view
        .known_institutional_beliefs(context.agent)
        .into_iter()
        .any(|belief| {
            matches!(
                (belief.claim, belief.source),
                (
                    InstitutionalClaim::Accusation { accused, theft, .. },
                    InstitutionalKnowledgeSource::RecordConsultation { .. }
                ) if accused == target && theft.quantity == terms.reward_quantity
            )
        });
    if !accusation_matches {
        return 0;
    }

    score_product(
        context.utility.bounty_posting_weight,
        reward_signal_from_quantity(terms.reward_quantity),
    )
}

fn post_notice_motive(
    context: &RankingContext<'_>,
    posting: worldwake_core::ArtifactPostingContext,
    topic: NoticeTopic,
) -> u32 {
    let NoticeTopic::ThreatWarning {
        place: warned_place,
    } = topic
    else {
        return 0;
    };
    if posting.issuing_authority.is_some() {
        return 0;
    }
    if context.view.effective_place(context.agent) != Some(posting.posting_place) {
        return 0;
    }
    let Some(thresholds) = context.thresholds else {
        return 0;
    };
    let threat_signal = threat_warning_signal_for_place(context.view, context.agent, warned_place);
    if threat_signal < thresholds.danger.high() {
        return 0;
    }
    score_product(context.utility.notice_posting_weight, threat_signal)
}

fn belief_pressure_from_state(
    state: &BelievedEntityState,
    current_tick: Tick,
    policy: &worldwake_core::BeliefConfidencePolicy,
) -> Permille {
    belief_pressure_from_source(state.source, state.observed_tick, current_tick, policy)
}

fn belief_pressure_from_source(
    source: PerceptionSource,
    observed_tick: Tick,
    current_tick: Tick,
    policy: &worldwake_core::BeliefConfidencePolicy,
) -> Permille {
    let staleness_ticks = current_tick.0.saturating_sub(observed_tick.0);
    belief_confidence(&source, staleness_ticks, policy)
}

fn investigation_motive(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    let Some(profile) = context.view.violation_disposition_profile(context.agent) else {
        return 0;
    };
    let base = u32::from(profile.investigation_motive_weight.value());
    let owns_evidence = candidate
        .evidence_entities
        .iter()
        .any(|&entity| context.view.believed_owner_of(entity) == Some(context.agent));
    let ownership_bonus = if owns_evidence {
        u32::from(profile.ownership_motive_bonus.value())
    } else {
        0
    };
    base.saturating_add(ownership_bonus)
}

fn patrol_motive(context: &RankingContext<'_>) -> u32 {
    let (Some(profile), Some(route)) = (
        context.view.patrol_profile(context.agent),
        context.view.patrol_route(context.agent),
    ) else {
        return 0;
    };
    if route.assigned_places.get(route.current_index).is_none() {
        return 0;
    }

    let base = u32::from(profile.patrol_motive_weight.value());
    let unresolved_thefts = patrol_unresolved_theft_count(context.view, context.agent);
    let relevant_offices = patrol_relevant_offices(context.view, context.agent, &route);
    let believed_vacancies = relevant_offices
        .iter()
        .filter(|office| {
            matches!(
                context.view.believed_office_holder(**office),
                worldwake_core::InstitutionalBeliefRead::Certain(None)
            )
        })
        .count() as u32;
    let believed_contests = relevant_offices
        .iter()
        .filter(|office| {
            matches!(
                context.view.believed_force_controller(**office),
                worldwake_core::InstitutionalBeliefRead::Certain((_, true))
            )
        })
        .count() as u32;

    base.saturating_mul(
        1u32.saturating_add(unresolved_thefts)
            .saturating_add(believed_vacancies)
            .saturating_add(believed_contests),
    )
}

fn patrol_unresolved_theft_count(view: &dyn GoalBeliefView, agent: EntityId) -> u32 {
    view.active_violation_records(agent)
        .into_iter()
        .filter(|record| matches!(record.kind, ViolationKind::SuspectedTheft { .. }))
        .count() as u32
}

fn patrol_relevant_offices(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    route: &worldwake_core::PatrolRoute,
) -> BTreeSet<EntityId> {
    let route_places = route
        .assigned_places
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    view.known_institutional_beliefs(agent)
        .into_iter()
        .filter_map(|belief| match belief.claim {
            InstitutionalClaim::OfficeHolder { office, .. }
            | InstitutionalClaim::ForceControl { office, .. } => {
                Some((office, view.office_data(office)?))
            }
            _ => None,
        })
        .filter_map(|(office, office_data)| {
            office_data
                .jurisdiction
                .iter()
                .any(|place| route_places.contains(place))
                .then_some(office)
        })
        .collect()
}

fn drive_score(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    weight: impl Fn(&UtilityProfile) -> Permille,
) -> u32 {
    match context.needs {
        Some(needs) => score_product(weight(context.utility), pressure(needs)),
        None => 0,
    }
}

fn self_consume_provenance(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> Option<RankedDriveGoalProvenance> {
    let factors = relevant_self_consume_factors(commodity, context);
    let base_priority_class = factors
        .iter()
        .map(|factor| classify_band(factor.pressure, &factor.band))
        .max()
        .unwrap_or(GoalPriorityClass::Background);
    let motive_inputs = factors
        .into_iter()
        .map(|factor| RankedDriveMotiveInput {
            drive: factor.drive,
            pressure: factor.pressure,
            weight: factor.weight,
            score: score_product(factor.weight, factor.pressure),
            relief_per_unit: factor.relief_per_unit,
            recovery_relevant: factor.recovery_relevant,
        })
        .collect::<Vec<_>>();
    (!motive_inputs.is_empty())
        .then(|| drive_provenance_from_inputs(context, base_priority_class, motive_inputs))
}

fn raid_target_motive(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    let GoalKind::RaidTarget { target } = candidate.key.kind else {
        unreachable!("raid_target_motive requires RaidTarget");
    };

    if is_bandit_raid_deterred_by_wounds(context.view, context.agent) {
        return 0;
    }

    CommodityKind::ALL
        .iter()
        .copied()
        .filter_map(|commodity| {
            let quantity = context.view.commodity_quantity(target, commodity);
            (quantity > Quantity(0)).then(|| {
                let mut simulated_holdings = context.holdings.clone();
                *simulated_holdings.entry(commodity).or_insert(0) += quantity.0;
                let breakdown = commodity_opportunity_score(
                    context.agent,
                    commodity,
                    context.view,
                    &simulated_holdings,
                    &context.local_alternatives,
                );
                commodity_shared_motive_score(commodity, breakdown, context)
                    .saturating_mul(quantity.0)
            })
        })
        .sum()
}

fn relevant_self_consume_factors(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> Vec<DriveFactor> {
    let Some(needs) = context.needs else {
        return Vec::new();
    };
    let Some(thresholds) = context.thresholds else {
        return Vec::new();
    };

    let Some(profile) = commodity.spec().consumable_profile else {
        return Vec::new();
    };

    let mut factors = Vec::new();
    if profile.hunger_relief_per_unit.value() > 0 {
        factors.push(DriveFactor {
            drive: RankedDriveKind::Hunger,
            pressure: needs.hunger,
            weight: context.utility.hunger_weight,
            band: thresholds.hunger,
            recovery_relevant: true,
            relief_per_unit: profile.hunger_relief_per_unit,
        });
    }
    if profile.thirst_relief_per_unit.value() > 0 {
        factors.push(DriveFactor {
            drive: RankedDriveKind::Thirst,
            pressure: needs.thirst,
            weight: context.utility.thirst_weight,
            band: thresholds.thirst,
            recovery_relevant: true,
            relief_per_unit: profile.thirst_relief_per_unit,
        });
    }
    factors
}

fn enterprise_score(commodity: CommodityKind, context: &RankingContext<'_>) -> u32 {
    let signal = opportunity_signal(
        context.view,
        context.agent,
        context.view.effective_place(context.agent),
        commodity,
    );
    score_product(context.utility.enterprise_weight, signal)
}

fn reward_signal_from_quantity(quantity: Quantity) -> Permille {
    let scaled = quantity.0.min(1000) as u16;
    Permille::new(scaled).unwrap_or_else(|_| Permille::new_unchecked(1000))
}

fn score_product(weight: Permille, pressure: Permille) -> u32 {
    u32::from(weight.value()) * u32::from(pressure.value())
}

fn holdings_from_view(view: &dyn GoalBeliefView, agent: EntityId) -> BTreeMap<CommodityKind, u32> {
    CommodityKind::ALL
        .iter()
        .copied()
        .map(|kind| (kind, view.commodity_quantity(agent, kind).0))
        .collect()
}

fn local_alternatives_from_view(
    view: &dyn GoalBeliefView,
    agent: EntityId,
) -> BTreeMap<CommodityKind, u32> {
    let Some(place) = view.effective_place(agent) else {
        return BTreeMap::new();
    };

    CommodityKind::ALL
        .iter()
        .copied()
        .map(|commodity| {
            let quantity = view
                .listed_sale_lots_at(place, commodity)
                .into_iter()
                .filter(|lot| view.seller_for_sale_lot(*lot) != Some(agent))
                .map(|lot| {
                    view.locally_observed_commodity_quantity(agent, lot, commodity)
                        .0
                })
                .sum();
            (commodity, quantity)
        })
        .collect()
}

fn treatment_priority(context: &RankingContext<'_>) -> GoalPriorityClass {
    context
        .thresholds
        .map_or(GoalPriorityClass::Background, |thresholds| {
            classify_band(
                derive_pain_pressure(context.view, context.agent),
                &thresholds.pain,
            )
        })
}

fn treatment_motive_score(context: &RankingContext<'_>) -> u32 {
    score_product(
        context.utility.pain_weight,
        derive_pain_pressure(context.view, context.agent),
    )
}

fn clamp_breakdown_to_permille(value: u32) -> Permille {
    Permille::new(value.min(1000) as u16).unwrap()
}

fn commodity_shared_priority(
    commodity: CommodityKind,
    breakdown: CommodityOpportunityBreakdown,
    context: &RankingContext<'_>,
) -> GoalPriorityClass {
    if breakdown.direct_survival_score > 0 {
        return self_consume_priority(commodity, context);
    }
    if breakdown.treatment_score > 0 {
        return treatment_priority(context);
    }
    if breakdown.enterprise_score > 0 || breakdown.indirect_recipe_score > 0 {
        GoalPriorityClass::Medium
    } else {
        GoalPriorityClass::Background
    }
}

fn commodity_shared_provenance(
    commodity: CommodityKind,
    breakdown: CommodityOpportunityBreakdown,
    context: &RankingContext<'_>,
) -> Option<RankedDriveGoalProvenance> {
    (breakdown.direct_survival_score > 0)
        .then(|| self_consume_provenance(commodity, context))
        .flatten()
}

fn commodity_shared_motive_score(
    commodity: CommodityKind,
    breakdown: CommodityOpportunityBreakdown,
    context: &RankingContext<'_>,
) -> u32 {
    if breakdown.direct_survival_score > 0 {
        return relevant_self_consume_factors(commodity, context)
            .into_iter()
            .map(|factor| score_product(factor.weight, factor.pressure))
            .max()
            .unwrap_or(0);
    }
    if breakdown.treatment_score > 0 {
        return treatment_motive_score(context);
    }

    let enterprise = enterprise_score(commodity, context);
    let indirect = score_product(
        context.utility.enterprise_weight,
        clamp_breakdown_to_permille(breakdown.indirect_recipe_score),
    );
    enterprise.max(indirect)
}

#[derive(Clone, Debug)]
struct RecipeOutputAssessment {
    commodity: CommodityKind,
    priority_class: GoalPriorityClass,
    motive_score: u32,
    provenance: Option<RankedDriveGoalProvenance>,
}

fn best_recipe_output_assessment(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
) -> Option<RecipeOutputAssessment> {
    let recipe = context.view.recipe_definition(recipe_id)?;
    recipe
        .outputs
        .iter()
        .map(|(commodity, quantity)| {
            let mut simulated_holdings = context.holdings.clone();
            *simulated_holdings.entry(*commodity).or_insert(0) += quantity.0;
            let breakdown = commodity_opportunity_score(
                context.agent,
                *commodity,
                context.view,
                &simulated_holdings,
                &context.local_alternatives,
            );
            RecipeOutputAssessment {
                commodity: *commodity,
                priority_class: commodity_shared_priority(*commodity, breakdown, context),
                motive_score: commodity_shared_motive_score(*commodity, breakdown, context),
                provenance: commodity_shared_provenance(*commodity, breakdown, context),
            }
        })
        .max_by(|left, right| {
            left.priority_class
                .cmp(&right.priority_class)
                .then_with(|| left.motive_score.cmp(&right.motive_score))
                .then_with(|| left.commodity.cmp(&right.commodity))
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankedGoalComparisonDimension {
    PriorityClass,
    Feasibility,
    MotiveScore,
    OpportunityStrength,
    ShareBeliefTopicOrder,
    GoalKindOrder,
    CommodityKey,
    EntityKey,
    PlaceKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RankedGoalComparison {
    pub winner: OpportunityKey,
    pub loser: OpportunityKey,
    pub decisive_dimension: RankedGoalComparisonDimension,
}

fn ranked_goal_ordering(
    left: &RankedGoal,
    right: &RankedGoal,
) -> (Ordering, Option<RankedGoalComparisonDimension>) {
    let ordering = right.priority_class.cmp(&left.priority_class);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::PriorityClass));
    }

    let ordering = left.feasibility.cmp(&right.feasibility);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::Feasibility));
    }

    let ordering = right.motive_score.cmp(&left.motive_score);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::MotiveScore));
    }

    let ordering = opportunity_strength(left)
        .cmp(&opportunity_strength(right))
        .reverse();
    if ordering != Ordering::Equal {
        return (
            ordering,
            Some(RankedGoalComparisonDimension::OpportunityStrength),
        );
    }

    let ordering = compare_share_belief_topics(&left.grounded.key.kind, &right.grounded.key.kind);
    if ordering != Ordering::Equal {
        return (
            ordering,
            Some(RankedGoalComparisonDimension::ShareBeliefTopicOrder),
        );
    }

    let ordering = goal_kind_discriminant(left.grounded.key.kind)
        .cmp(&goal_kind_discriminant(right.grounded.key.kind));
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::GoalKindOrder));
    }

    let ordering = left
        .grounded
        .key
        .commodity
        .cmp(&right.grounded.key.commodity);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::CommodityKey));
    }

    let ordering = left.grounded.key.entity.cmp(&right.grounded.key.entity);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::EntityKey));
    }

    let ordering = left.grounded.key.place.cmp(&right.grounded.key.place);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::PlaceKey));
    }

    (Ordering::Equal, None)
}

pub(crate) fn explain_ranked_goal_order(
    left: &RankedGoal,
    right: &RankedGoal,
) -> Option<RankedGoalComparison> {
    let (ordering, decisive_dimension) = ranked_goal_ordering(left, right);
    let decisive_dimension = decisive_dimension?;
    let (winner, loser) = match ordering {
        Ordering::Less => (
            OpportunityKey {
                goal_key: left.grounded.key,
                anchor: left.grounded.anchor,
            },
            OpportunityKey {
                goal_key: right.grounded.key,
                anchor: right.grounded.anchor,
            },
        ),
        Ordering::Greater => (
            OpportunityKey {
                goal_key: right.grounded.key,
                anchor: right.grounded.anchor,
            },
            OpportunityKey {
                goal_key: left.grounded.key,
                anchor: left.grounded.anchor,
            },
        ),
        Ordering::Equal => return None,
    };
    Some(RankedGoalComparison {
        winner,
        loser,
        decisive_dimension,
    })
}

pub(crate) fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
    ranked_goal_ordering(left, right).0
}

fn compare_share_belief_topics(left: &GoalKind, right: &GoalKind) -> Ordering {
    match (left, right) {
        (
            GoalKind::ShareBelief {
                topic: left_topic, ..
            },
            GoalKind::ShareBelief {
                topic: right_topic, ..
            },
        ) => share_belief_topic_priority(left_topic)
            .cmp(&share_belief_topic_priority(right_topic))
            .then_with(|| match (left_topic, right_topic) {
                (
                    TellTopic::InstitutionalClaim { claim: left_claim },
                    TellTopic::InstitutionalClaim { claim: right_claim },
                ) => institutional_claim_priority(left_claim)
                    .cmp(&institutional_claim_priority(right_claim))
                    .then_with(|| left_claim.cmp(right_claim)),
                _ => left_topic.cmp(right_topic),
            }),
        _ => Ordering::Equal,
    }
}

fn opportunity_strength(goal: &RankedGoal) -> u32 {
    match (&goal.grounded.key.kind, goal.provenance.as_ref()) {
        (
            GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::AcquireCommodity {
                purpose: CommodityPurpose::SelfConsume,
                ..
            }
            | GoalKind::AcquireCommodity {
                purpose: CommodityPurpose::RecipeInput(_),
                ..
            }
            | GoalKind::ProduceCommodity { .. },
            Some(RankedGoalProvenance::Drive(provenance)),
        ) => provenance
            .motive_inputs
            .iter()
            .map(|input| {
                u32::from(input.pressure.value()) * u32::from(input.relief_per_unit.value())
            })
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}

fn share_belief_topic_priority(topic: &TellTopic) -> u8 {
    match topic {
        TellTopic::InstitutionalClaim { .. } => 0,
        TellTopic::SocialObservation { .. } => 1,
        TellTopic::EntityBelief { .. } => 2,
    }
}

fn institutional_claim_priority(claim: &worldwake_core::InstitutionalClaim) -> u8 {
    match claim {
        worldwake_core::InstitutionalClaim::ForceControl { .. } => 0,
        worldwake_core::InstitutionalClaim::OfficeHolder { .. } => 1,
        worldwake_core::InstitutionalClaim::FactionRallyPoint { .. } => 2,
        worldwake_core::InstitutionalClaim::SupportDeclaration { .. } => 3,
        worldwake_core::InstitutionalClaim::FactionMembership { .. } => 4,
        worldwake_core::InstitutionalClaim::Accusation { .. } => 5,
        worldwake_core::InstitutionalClaim::Verdict { .. } => 6,
    }
}

fn goal_kind_discriminant(kind: GoalKind) -> u8 {
    match kind {
        GoalKind::ConsumeOwnedCommodity { .. } => 0,
        GoalKind::AcquireCommodity { .. } => 1,
        GoalKind::Sleep => 2,
        GoalKind::Relieve => 3,
        GoalKind::Wash => 4,
        GoalKind::EngageHostile { .. } => 5,
        GoalKind::RaidTarget { .. } => 6,
        GoalKind::ReduceDanger => 7,
        GoalKind::RegroupWithFaction { .. } => 8,
        GoalKind::EstablishBanditCamp { .. } => 9,
        GoalKind::TreatWounds { .. } => 10,
        GoalKind::ProduceCommodity { .. } => 11,
        GoalKind::SellCommodity { .. } => 12,
        GoalKind::RestockCommodity { .. } => 13,
        GoalKind::MoveCargo { .. } => 14,
        GoalKind::LootCorpse { .. } => 15,
        GoalKind::BuryCorpse { .. } => 16,
        GoalKind::FulfillBounty { .. } => 17,
        GoalKind::ShareBelief { .. } => 18,
        GoalKind::ClaimOffice { .. } => 19,
        GoalKind::SupportCandidateForOffice { .. } => 20,
        GoalKind::InvestigateViolation { .. } => 21,
        GoalKind::Patrol { .. } => 22,
        GoalKind::StealItem { .. } => 23,
        GoalKind::Accuse { .. } => 24,
        GoalKind::PunishAccused { .. } => 25,
        GoalKind::PostBounty { .. } => 26,
        GoalKind::PostNotice { .. } => 27,
        GoalKind::SearchForMissing { .. } => 28,
        GoalKind::ReportMissing { .. } => 29,
        GoalKind::EscortToSafety { .. } => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RankingContext, apply_competition_discount, apply_source_reliability_discount,
        build_decision_context, rank_candidates,
    };
    use crate::{
        GoalKey, GoalKind, GoalPriorityClass, GroundedGoal, RankedDriveKind, RankedGoalProvenance,
        RankedPriorityAdjustment,
        decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDomain, ArtifactKind, ArtifactPostingContext, ArtifactState, BeliefConfidencePolicy,
        BelievedActivity, BelievedArtifactState, BelievedBountyTerms, BelievedEntityState,
        BelievedInstitutionalClaim, BodyCostPerTick, BodyPart, BountyTarget, BountyTerms,
        CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        CommodityValuationProfile, DemandObservation, DemandObservationReason, DeprivationKind,
        DriveThresholds, EffectiveRight, EntityId, EntityKind, EpistemicDispositionProfile,
        HomeostaticNeeds, InTransitOnEdge, InstitutionalBeliefRead, InstitutionalClaim,
        InstitutionalKnowledgeSource, JusticeDispositionProfile, LoadUnits, MerchandiseProfile,
        MetabolismProfile, NoticeTopic, OfficeData, OpportunityAnchor, PatrolProfile, PatrolRoute,
        PerceptionSource, Permille, PreferenceProfile, ProofRequirement, PunishmentKind, Quantity,
        RecipeId, RecordedViolation, ReliabilityRecord, ResourceSource, RewardSource, RightKind,
        RouteExperience, SourceKey, SourceReliability, TellTopic, TheftDispositionProfile,
        TheftFacts, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, UtilityProfile,
        ViolationId, ViolationKind, WorkstationTag, Wound, WoundCause, WoundId, belief_confidence,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, DurationExpr, RecipeDefinition, RuntimeBeliefView,
    };

    #[derive(Clone, Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        place_entities: BTreeMap<EntityId, Vec<EntityId>>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        confidence_policies: BTreeMap<EntityId, BeliefConfidencePolicy>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        courage: BTreeMap<EntityId, Permille>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        commodity_valuation_profiles: BTreeMap<EntityId, CommodityValuationProfile>,
        route_experiences: BTreeMap<EntityId, RouteExperience>,
        source_reliabilities: BTreeMap<EntityId, SourceReliability>,
        preference_profiles: BTreeMap<EntityId, PreferenceProfile>,
        theft_profiles: BTreeMap<EntityId, TheftDispositionProfile>,
        justice_profiles: BTreeMap<EntityId, JusticeDispositionProfile>,
        epistemic_profiles: BTreeMap<EntityId, EpistemicDispositionProfile>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        known_recipes: BTreeMap<EntityId, Vec<RecipeId>>,
        recipe_definitions: BTreeMap<RecipeId, RecipeDefinition>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        institutional_claims: BTreeMap<EntityId, Vec<BelievedInstitutionalClaim>>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        listed_sale_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        sale_lot_sellers: BTreeMap<EntityId, EntityId>,
        matching_workstations: BTreeMap<(EntityId, WorkstationTag), Vec<EntityId>>,
        office_data: BTreeMap<EntityId, OfficeData>,
        office_holder_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        force_controller_beliefs:
            BTreeMap<EntityId, InstitutionalBeliefRead<(Option<EntityId>, bool)>>,
        believed_rights: BTreeMap<(EntityId, EntityId), Vec<EffectiveRight>>,
        loyalties: BTreeMap<(EntityId, EntityId), Permille>,
        factions_by_member: BTreeMap<EntityId, Vec<EntityId>>,
        bandit_flee_thresholds: BTreeMap<EntityId, Permille>,
        patrol_profiles: BTreeMap<EntityId, PatrolProfile>,
        patrol_routes: BTreeMap<EntityId, PatrolRoute>,
        active_violation_records: BTreeMap<EntityId, Vec<RecordedViolation>>,
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.place_entities.get(&place).cloned().unwrap_or_default()
        }
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }
        fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
            self.institutional_claims
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
        fn believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity> {
            self.beliefs.values().find_map(|beliefs| {
                beliefs.iter().find_map(|(subject, state)| {
                    (*subject == entity)
                        .then_some(state.believed_activity.as_ref())
                        .flatten()
                })
            })
        }
        fn agents_active_at(
            &self,
            place: EntityId,
            domain: ActionDomain,
            target: Option<EntityId>,
        ) -> Vec<EntityId> {
            let mut entities = self
                .beliefs
                .values()
                .flat_map(|beliefs| beliefs.iter())
                .filter_map(|(entity, state)| {
                    (state.last_known_place == Some(place)
                        && state.believed_activity.as_ref().is_some_and(|activity| {
                            activity.action_domain == domain
                                && (target.is_none() || activity.target == target)
                        }))
                    .then_some(*entity)
                })
                .collect::<Vec<_>>();
            entities.sort();
            entities.dedup();
            entities
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }
        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }
        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }
        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
            self.bandit_flee_thresholds.get(&faction).copied()
        }
        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
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
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
            self.believed_rights
                .get(&(actor, entity))
                .cloned()
                .unwrap_or_default()
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courage.get(&agent).copied()
        }
        fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy {
            *self
                .confidence_policies
                .get(&agent)
                .expect("tests must seed a confidence policy for the acting agent")
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn commodity_valuation_profile(
            &self,
            agent: EntityId,
        ) -> Option<CommodityValuationProfile> {
            self.commodity_valuation_profiles.get(&agent).copied()
        }
        fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
            self.route_experiences.get(&agent).cloned()
        }
        fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
            self.source_reliabilities.get(&agent).cloned()
        }
        fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
            self.preference_profiles.get(&agent).copied()
        }
        fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
            self.patrol_profiles.get(&agent).cloned()
        }
        fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
            self.patrol_routes.get(&agent).cloned()
        }
        fn theft_disposition_profile(&self, agent: EntityId) -> Option<TheftDispositionProfile> {
            self.theft_profiles.get(&agent).cloned()
        }
        fn epistemic_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<EpistemicDispositionProfile> {
            self.epistemic_profiles.get(&agent).cloned()
        }
        fn justice_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<JusticeDispositionProfile> {
            self.justice_profiles.get(&agent).cloned()
        }
        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }
        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }
        fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
            self.factions_by_member
                .get(&entity)
                .cloned()
                .unwrap_or_default()
        }
        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }
        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }
        fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.listed_sale_lots
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }
        fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
            self.sale_lot_sellers.get(&lot).copied()
        }
        fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
            self.known_recipes.get(&agent).cloned().unwrap_or_default()
        }
        fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
            self.recipe_definitions.get(&recipe).cloned()
        }
        fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
            self.matching_workstations
                .get(&(place, tag))
                .cloned()
                .unwrap_or_default()
        }
        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
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
        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
            self.loyalties.get(&(subject, target)).copied()
        }
        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
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
        fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
            self.active_violation_records
                .get(&agent)
                .cloned()
                .unwrap_or_default()
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

    fn patrol_profile(weight: u16) -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks: 5,
            dwell_vigilance_scale_ticks: 5,
            vigilance: pm(500),
            route_adaptation_sensitivity: pm(400),
            patrol_motive_weight: pm(weight),
        }
    }

    fn patrol_route(place: EntityId) -> PatrolRoute {
        PatrolRoute {
            assigned_places: vec![place],
            current_index: 0,
        }
    }

    fn suspected_theft_record(id: u64, place: EntityId) -> RecordedViolation {
        RecordedViolation {
            id: ViolationId(id),
            kind: ViolationKind::SuspectedTheft {
                theft: TheftFacts {
                    missing_entity: entity(800 + id as u32),
                    expected_place: place,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                },
                suspect: None,
            },
            observed_tick: Tick(1),
            resolved_tick: None,
            expires_tick: Tick(50),
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

    fn believed_state(observed_tick: u64, source: PerceptionSource) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(entity(99)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(observed_tick),
            source,
        }
    }

    fn wound_with_bleed(severity: u16, bleed_rate: u16) -> Wound {
        Wound {
            id: WoundId(u64::from(severity)),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(DeprivationKind::Starvation),
            severity: pm(severity),
            inflicted_at: Tick(1),
            bleed_rate_per_tick: pm(bleed_rate),
        }
    }

    fn wound(severity: u16) -> Wound {
        wound_with_bleed(severity, 0)
    }

    fn goal(kind: GoalKind) -> GroundedGoal {
        GroundedGoal {
            anchor: OpportunityAnchor::None,
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        }
    }

    fn goal_at_place(kind: GoalKind, place: EntityId) -> GroundedGoal {
        GroundedGoal {
            anchor: OpportunityAnchor::Place(place),
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([place]),
        }
    }

    fn goal_at_place_with_sources(
        kind: GoalKind,
        place: EntityId,
        evidence_entities: BTreeSet<EntityId>,
    ) -> GroundedGoal {
        GroundedGoal {
            anchor: OpportunityAnchor::Place(place),
            key: GoalKey::from(kind),
            evidence_entities,
            evidence_places: BTreeSet::from([place]),
        }
    }

    fn observed_activity_state(
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: Some(BelievedActivity {
                action_domain: domain,
                target,
                observed_tick: Tick(9),
            }),
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(9),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn believed_bounty_state(
        issuer: EntityId,
        claim_place: EntityId,
        target: BountyTarget,
        reward_quantity: u32,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(claim_place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: Some(BelievedArtifactState {
                kind: ArtifactKind::Bounty,
                state: ArtifactState::Active,
                issuer,
                expires_at: None,
                bounty_terms: Some(BelievedBountyTerms {
                    target,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(reward_quantity),
                    claim_place,
                }),
                notice_topic: None,
                observed_tick: Tick(9),
            }),
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(9),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn utility() -> UtilityProfile {
        UtilityProfile {
            hunger_weight: pm(900),
            thirst_weight: pm(800),
            fatigue_weight: pm(700),
            bladder_weight: pm(600),
            dirtiness_weight: pm(500),
            pain_weight: pm(400),
            danger_weight: pm(300),
            enterprise_weight: pm(200),
            social_weight: pm(150),
            activity_awareness_weight: pm(200),
            side_benefit_weight: pm(100),
            bounty_posting_weight: pm(0),
            notice_posting_weight: pm(0),
            courage: pm(500),
            care_weight: pm(200),
        }
    }

    fn current_tick() -> Tick {
        Tick(10)
    }

    fn base_view(agent: EntityId) -> TestBeliefView {
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        let place = entity(99);
        view.effective_places.insert(agent, place);
        view.place_entities.insert(place, vec![agent]);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
        );
        view.thresholds.insert(agent, DriveThresholds::default());
        view.confidence_policies
            .insert(agent, BeliefConfidencePolicy::default());
        view
    }

    fn teach_recipe(
        view: &mut TestBeliefView,
        agent: EntityId,
        recipe: RecipeDefinition,
    ) -> RecipeId {
        let recipe_id = RecipeId(view.recipe_definitions.len() as u32);
        view.known_recipes.entry(agent).or_default().push(recipe_id);
        view.recipe_definitions.insert(recipe_id, recipe);
        recipe_id
    }

    fn add_home_facility(view: &mut TestBeliefView, market: EntityId, facility: EntityId) {
        view.alive.insert(facility);
        view.entity_kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(facility, market);
        view.place_entities
            .entry(market)
            .or_default()
            .push(facility);
    }

    fn source_reliability_record(
        successful_acquisitions: u16,
        failed_attempts: u16,
    ) -> ReliabilityRecord {
        ReliabilityRecord {
            successful_acquisitions,
            failed_attempts,
            last_attempt_tick: current_tick(),
        }
    }

    #[test]
    fn crime_goals_use_profile_driven_motive_scores() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.theft_profiles.insert(
            agent,
            TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(700),
                witness_risk_penalty: pm(150),
            },
        );
        view.justice_profiles.insert(
            agent,
            JusticeDispositionProfile {
                accusation_motive_weight: pm(640),
                fine_severity: pm(500),
            },
        );
        let place = entity(99);
        view.entity_kinds.insert(entity(2), EntityKind::ItemLot);
        view.entity_kinds.insert(entity(8), EntityKind::Agent);
        view.entity_kinds.insert(entity(9), EntityKind::Agent);
        view.alive.insert(entity(8));
        view.alive.insert(entity(9));
        view.place_entities
            .insert(place, vec![agent, entity(8), entity(9)]);

        let outcome = rank(
            &[
                goal(GoalKind::StealItem {
                    target_item: entity(2),
                }),
                goal(GoalKind::Accuse {
                    crime_register: entity(7),
                    accused: entity(3),
                    violation_id: ViolationId(1),
                }),
                goal(GoalKind::PunishAccused {
                    office: entity(6),
                    accused: entity(4),
                    accusation_entry: worldwake_core::RecordEntryId(1),
                    punishment: PunishmentKind::Exile {
                        from_faction: entity(5),
                    },
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert_eq!(outcome.ranked.len(), 3);
        let steal = outcome
            .ranked
            .iter()
            .find(|ranked| {
                ranked.grounded.key.kind
                    == GoalKind::StealItem {
                        target_item: entity(2),
                    }
            })
            .unwrap();
        assert_eq!(steal.priority_class, GoalPriorityClass::Low);
        assert_eq!(steal.motive_score, 400);

        let accuse = outcome
            .ranked
            .iter()
            .find(|ranked| {
                ranked.grounded.key.kind
                    == GoalKind::Accuse {
                        crime_register: entity(7),
                        accused: entity(3),
                        violation_id: ViolationId(1),
                    }
            })
            .unwrap();
        assert_eq!(accuse.priority_class, GoalPriorityClass::Low);
        assert_eq!(accuse.motive_score, 640);

        let punish = outcome
            .ranked
            .iter()
            .find(|ranked| {
                ranked.grounded.key.kind
                    == GoalKind::PunishAccused {
                        office: entity(6),
                        accused: entity(4),
                        accusation_entry: worldwake_core::RecordEntryId(1),
                        punishment: PunishmentKind::Exile {
                            from_faction: entity(5),
                        },
                    }
            })
            .unwrap();
        assert_eq!(punish.priority_class, GoalPriorityClass::Low);
        assert_eq!(punish.motive_score, 640);
    }

    #[test]
    fn post_bounty_goal_has_non_zero_motive_for_live_accusation_case() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let seat = entity(99);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: seat,
            commodity: CommodityKind::Bread,
            quantity: Quantity(6),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id: ViolationId(12),
            theft,
            effective_tick: Tick(3),
        };
        let mut view = base_view(agent);
        view.alive.insert(accused);
        view.office_data.insert(
            office,
            OfficeData {
                title: "Magistrate".to_string(),
                seat,
                jurisdiction: BTreeSet::from([seat]),
                succession_law: worldwake_core::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 10,
                vacancy_since: None,
            },
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.institutional_claims.insert(
            agent,
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: entity(4),
                    entry_id: worldwake_core::RecordEntryId(21),
                },
                learned_tick: Tick(4),
                learned_at: Some(seat),
            }],
        );
        view.believed_rights.insert(
            (agent, accused),
            vec![EffectiveRight {
                kind: RightKind::JurisdictionalAuthority,
                via: Some(office),
            }],
        );

        let mut utility = utility();
        utility.bounty_posting_weight = pm(700);

        let outcome = rank(
            &[goal(GoalKind::PostBounty {
                posting: ArtifactPostingContext {
                    posting_place: seat,
                    issuing_authority: Some(office),
                    expires_at: None,
                    jurisdiction: Some(seat),
                },
                terms: BountyTerms {
                    target: BountyTarget::EliminateEntity { target: accused },
                    proof_requirement: ProofRequirement::PhysicalEvidence,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(6),
                    reward_source: RewardSource::InstitutionalTreasury {
                        treasury_entity: office,
                    },
                    claim_place: seat,
                },
            })],
            &view,
            agent,
            current_tick(),
            &utility,
        );

        assert_eq!(outcome.zero_motive, Vec::<GoalKey>::new());
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(outcome.ranked[0].motive_score, 4_200);
    }

    #[test]
    fn post_bounty_goal_is_zero_motive_when_bounty_weight_is_zero() {
        let agent = entity(1);
        let accused = entity(2);
        let office = entity(3);
        let seat = entity(99);
        let theft = TheftFacts {
            missing_entity: entity(20),
            expected_place: seat,
            commodity: CommodityKind::Bread,
            quantity: Quantity(6),
        };
        let claim = InstitutionalClaim::Accusation {
            accuser: entity(9),
            accused,
            violation_id: ViolationId(13),
            theft,
            effective_tick: Tick(3),
        };
        let mut view = base_view(agent);
        view.alive.insert(accused);
        view.office_data.insert(
            office,
            OfficeData {
                title: "Magistrate".to_string(),
                seat,
                jurisdiction: BTreeSet::from([seat]),
                succession_law: worldwake_core::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 10,
                vacancy_since: None,
            },
        );
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(Some(agent)));
        view.institutional_claims.insert(
            agent,
            vec![BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: entity(4),
                    entry_id: worldwake_core::RecordEntryId(22),
                },
                learned_tick: Tick(4),
                learned_at: Some(seat),
            }],
        );
        view.believed_rights.insert(
            (agent, accused),
            vec![EffectiveRight {
                kind: RightKind::JurisdictionalAuthority,
                via: Some(office),
            }],
        );

        let mut utility = utility();
        utility.bounty_posting_weight = pm(0);
        let goal_kind = GoalKind::PostBounty {
            posting: ArtifactPostingContext {
                posting_place: seat,
                issuing_authority: Some(office),
                expires_at: None,
                jurisdiction: Some(seat),
            },
            terms: BountyTerms {
                target: BountyTarget::EliminateEntity { target: accused },
                proof_requirement: ProofRequirement::PhysicalEvidence,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(6),
                reward_source: RewardSource::InstitutionalTreasury {
                    treasury_entity: office,
                },
                claim_place: seat,
            },
        };

        let outcome = rank(&[goal(goal_kind)], &view, agent, current_tick(), &utility);

        assert!(outcome.ranked.is_empty());
        assert_eq!(outcome.zero_motive, vec![GoalKey::from(goal_kind)]);
    }

    #[test]
    fn post_notice_goal_has_non_zero_motive_for_live_high_danger_case() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(99);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.alive.insert(hostile);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(hostile, place);
        view.place_entities.insert(place, vec![agent, hostile]);
        view.hostiles.insert(agent, vec![hostile]);
        view.attackers.insert(agent, vec![hostile]);
        view.thresholds.insert(agent, thresholds);

        let mut utility = utility();
        utility.notice_posting_weight = pm(700);

        let outcome = rank(
            &[goal_at_place(
                GoalKind::PostNotice {
                    posting: ArtifactPostingContext {
                        posting_place: place,
                        issuing_authority: None,
                        expires_at: None,
                        jurisdiction: Some(place),
                    },
                    topic: NoticeTopic::ThreatWarning { place },
                },
                place,
            )],
            &view,
            agent,
            current_tick(),
            &utility,
        );

        let expected_motive = super::score_product(
            utility.notice_posting_weight,
            super::derive_danger_pressure(&view, agent),
        );
        assert_eq!(outcome.zero_motive, Vec::<GoalKey>::new());
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(outcome.ranked[0].motive_score, expected_motive);
    }

    #[test]
    fn post_notice_goal_has_non_zero_motive_for_remote_warned_place_from_belief() {
        let agent = entity(1);
        let hostile = entity(2);
        let posting_place = entity(99);
        let warned_place = entity(100);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.thresholds.insert(agent, thresholds);
        view.beliefs.insert(
            agent,
            vec![(
                hostile,
                observed_activity_state(warned_place, ActionDomain::Combat, Some(agent)),
            )],
        );

        let mut utility = utility();
        utility.notice_posting_weight = pm(700);

        let outcome = rank(
            &[goal_at_place(
                GoalKind::PostNotice {
                    posting: ArtifactPostingContext {
                        posting_place,
                        issuing_authority: None,
                        expires_at: None,
                        jurisdiction: Some(posting_place),
                    },
                    topic: NoticeTopic::ThreatWarning {
                        place: warned_place,
                    },
                },
                posting_place,
            )],
            &view,
            agent,
            current_tick(),
            &utility,
        );

        let expected_motive = super::score_product(
            utility.notice_posting_weight,
            crate::route_threat::threat_warning_signal_for_place(&view, agent, warned_place),
        );
        assert_eq!(outcome.zero_motive, Vec::<GoalKey>::new());
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(outcome.ranked[0].motive_score, expected_motive);
    }

    #[test]
    fn post_notice_goal_is_zero_motive_when_notice_weight_is_zero() {
        let agent = entity(1);
        let hostile = entity(2);
        let place = entity(99);
        let mut view = base_view(agent);
        view.alive.insert(hostile);
        view.entity_kinds.insert(hostile, EntityKind::Agent);
        view.effective_places.insert(hostile, place);
        view.place_entities.insert(place, vec![agent, hostile]);
        view.hostiles.insert(agent, vec![hostile]);
        view.attackers.insert(agent, vec![hostile]);

        let mut utility = utility();
        utility.notice_posting_weight = pm(0);
        let goal_kind = GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place: place,
                issuing_authority: None,
                expires_at: None,
                jurisdiction: Some(place),
            },
            topic: NoticeTopic::ThreatWarning { place },
        };

        let outcome = rank(
            &[goal_at_place(goal_kind, place)],
            &view,
            agent,
            current_tick(),
            &utility,
        );

        assert!(outcome.ranked.is_empty());
        assert_eq!(outcome.zero_motive, vec![GoalKey::from(goal_kind)]);
    }

    #[test]
    fn post_notice_goal_is_zero_motive_without_live_threat_substrate() {
        let agent = entity(1);
        let place = entity(99);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.thresholds.insert(agent, thresholds);

        let mut utility = utility();
        utility.notice_posting_weight = pm(700);
        let goal_kind = GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place: place,
                issuing_authority: None,
                expires_at: None,
                jurisdiction: Some(place),
            },
            topic: NoticeTopic::ThreatWarning { place },
        };

        let outcome = rank(
            &[goal_at_place(goal_kind, place)],
            &view,
            agent,
            current_tick(),
            &utility,
        );

        assert!(outcome.ranked.is_empty());
        assert_eq!(outcome.zero_motive, vec![GoalKey::from(goal_kind)]);
    }

    #[test]
    fn theft_goal_is_zero_motive_when_witness_penalty_cancels_profile_weight() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let place = entity(99);

        view.theft_profiles.insert(
            agent,
            TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(600),
                witness_risk_penalty: pm(300),
            },
        );
        view.entity_kinds.insert(entity(2), EntityKind::ItemLot);
        view.entity_kinds.insert(entity(8), EntityKind::Agent);
        view.entity_kinds.insert(entity(9), EntityKind::Agent);
        view.alive.insert(entity(8));
        view.alive.insert(entity(9));
        view.place_entities
            .insert(place, vec![agent, entity(2), entity(8), entity(9)]);

        let outcome = rank(
            &[goal(GoalKind::StealItem {
                target_item: entity(2),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.ranked.is_empty());
        assert_eq!(
            outcome.zero_motive,
            vec![GoalKey::from(GoalKind::StealItem {
                target_item: entity(2),
            })]
        );
    }

    #[test]
    fn patrol_goal_uses_patrol_profile_weight_as_base_motive() {
        let agent = entity(1);
        let place = entity(99);
        let mut view = base_view(agent);
        view.patrol_profiles.insert(agent, patrol_profile(550));
        view.patrol_routes.insert(agent, patrol_route(place));

        let ranked = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(ranked[0].motive_score, 550);
    }

    #[test]
    fn patrol_goal_scales_with_unresolved_theft_records() {
        let agent = entity(1);
        let place = entity(99);
        let mut view = base_view(agent);
        view.patrol_profiles.insert(agent, patrol_profile(200));
        view.patrol_routes.insert(agent, patrol_route(place));

        let baseline = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        view.active_violation_records.insert(
            agent,
            vec![
                suspected_theft_record(1, place),
                suspected_theft_record(2, place),
                suspected_theft_record(3, place),
            ],
        );
        let escalated = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(baseline[0].motive_score, 200);
        assert_eq!(escalated[0].motive_score, 800);
    }

    #[test]
    fn patrol_goal_scales_with_vacancy_and_contested_beliefs_on_route() {
        let agent = entity(1);
        let place = entity(99);
        let office = entity(77);
        let mut view = base_view(agent);
        view.patrol_profiles.insert(agent, patrol_profile(150));
        view.patrol_routes.insert(agent, patrol_route(place));
        view.entity_kinds.insert(office, EntityKind::Office);
        view.office_data.insert(
            office,
            OfficeData {
                title: "Captain".to_string(),
                seat: place,
                jurisdiction: BTreeSet::from([place]),
                succession_law: worldwake_core::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 10,
                vacancy_since: Some(Tick(2)),
            },
        );
        view.institutional_claims.insert(
            agent,
            vec![
                BelievedInstitutionalClaim {
                    claim: InstitutionalClaim::OfficeHolder {
                        office,
                        holder: None,
                        effective_tick: Tick(2),
                    },
                    source: InstitutionalKnowledgeSource::WitnessedEvent,
                    learned_tick: Tick(2),
                    learned_at: Some(place),
                },
                BelievedInstitutionalClaim {
                    claim: InstitutionalClaim::ForceControl {
                        office,
                        controller: Some(entity(30)),
                        contested: true,
                        effective_tick: Tick(2),
                    },
                    source: InstitutionalKnowledgeSource::WitnessedEvent,
                    learned_tick: Tick(2),
                    learned_at: Some(place),
                },
            ],
        );

        let baseline_view = view.clone();
        let baseline = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &baseline_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));
        view.force_controller_beliefs.insert(
            office,
            InstitutionalBeliefRead::Certain((Some(entity(30)), true)),
        );

        let escalated = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(baseline[0].motive_score, 150);
        assert_eq!(escalated[0].motive_score, 450);
    }

    #[test]
    fn patrol_goal_does_not_escalate_without_local_patrol_beliefs() {
        let agent = entity(1);
        let place = entity(99);
        let office = entity(77);
        let mut view = base_view(agent);
        view.patrol_profiles.insert(agent, patrol_profile(175));
        view.patrol_routes.insert(agent, patrol_route(place));
        view.entity_kinds.insert(office, EntityKind::Office);
        view.office_data.insert(
            office,
            OfficeData {
                title: "Captain".to_string(),
                seat: place,
                jurisdiction: BTreeSet::from([place]),
                succession_law: worldwake_core::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 10,
                vacancy_since: Some(Tick(2)),
            },
        );

        let ranked = rank(
            &[goal_at_place(GoalKind::Patrol { place }, place)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 175);
    }

    /// Test helper: builds `DecisionContext` from the view and delegates to `rank_candidates`.
    fn rank(
        candidates: &[GroundedGoal],
        view: &TestBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &UtilityProfile,
    ) -> super::RankingOutcome {
        let dc = build_decision_context(view, agent);
        rank_candidates(candidates, view, agent, current_tick, utility, &dc)
    }

    #[test]
    fn hunger_candidate_becomes_critical_and_uses_weight_times_pressure() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
    }

    #[test]
    fn enterprise_goals_are_capped_at_medium_even_with_full_signal() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank(
            &[goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn production_competition_discount_applies_to_restock_goals() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let competitor_a = entity(10);
        let competitor_b = entity(11);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        view.beliefs.insert(
            agent,
            vec![
                (
                    competitor_a,
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    competitor_b,
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
            ],
        );

        let ranked = rank(
            &[goal_at_place(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                market,
            )],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].motive_score, 120_000);
        assert_eq!(
            ranked[0].competition_discount,
            Some(CompetitionDiscount {
                observed_competitors: vec![competitor_a, competitor_b],
                domain: ActionDomain::Production,
                effective_discount: pm(400),
                pre_discount_motive: 200_000,
                post_discount_motive: 120_000,
            })
        );
    }

    #[test]
    fn production_competition_discount_caps_at_three_competitors() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        let recipe_id = teach_recipe(
            &mut view,
            agent,
            RecipeDefinition {
                name: "Bake Bread".to_string(),
                inputs: vec![(CommodityKind::Firewood, Quantity(1))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                work_ticks: NonZeroU32::new(3).unwrap(),
                required_workstation_tag: Some(WorkstationTag::Mill),
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::zero(),
            },
        );
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        view.beliefs.insert(
            agent,
            vec![
                (
                    entity(10),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    entity(11),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    entity(12),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    entity(13),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
            ],
        );

        let ranked = rank(
            &[goal_at_place(
                GoalKind::ProduceCommodity { recipe_id },
                market,
            )],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        let discount = ranked[0].competition_discount.as_ref().unwrap();
        assert_eq!(discount.effective_discount, pm(600));
        assert_eq!(discount.pre_discount_motive, 90_000);
        assert_eq!(discount.post_discount_motive, 36_000);
        assert_eq!(discount.observed_competitors.len(), 4);
    }

    #[test]
    fn production_competition_discount_respects_zero_awareness_weight() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let competitor = entity(10);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        view.beliefs.insert(
            agent,
            vec![(
                competitor,
                observed_activity_state(market, ActionDomain::Production, None),
            )],
        );
        let mut zero_awareness = utility();
        zero_awareness.activity_awareness_weight = pm(0);

        let ranked = rank(
            &[goal_at_place(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                market,
            )],
            &view,
            agent,
            current_tick(),
            &zero_awareness,
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 200_000);
        assert_eq!(
            ranked[0].competition_discount,
            Some(CompetitionDiscount {
                observed_competitors: vec![competitor],
                domain: ActionDomain::Production,
                effective_discount: pm(0),
                pre_discount_motive: 200_000,
                post_discount_motive: 200_000,
            })
        );
    }

    #[test]
    fn production_competition_discount_floors_positive_motive_at_one() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![
                (
                    entity(10),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    entity(11),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
                (
                    entity(12),
                    observed_activity_state(market, ActionDomain::Production, None),
                ),
            ],
        );
        let mut aggressive_awareness = utility();
        aggressive_awareness.activity_awareness_weight = pm(500);
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &aggressive_awareness,
            build_decision_context(&view, agent),
        );

        let discount = apply_competition_discount(
            &goal_at_place(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                market,
            ),
            &context,
            1,
        )
        .unwrap();

        assert_eq!(discount.effective_discount, pm(1000));
        assert_eq!(discount.pre_discount_motive, 1);
        assert_eq!(discount.post_discount_motive, 1);
    }

    #[test]
    fn acquire_commodity_is_not_discounted_by_observed_production_activity() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        view.beliefs.insert(
            agent,
            vec![(
                entity(10),
                observed_activity_state(market, ActionDomain::Production, None),
            )],
        );

        let ranked = rank(
            &[goal_at_place(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::Restock,
                },
                market,
            )],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 200_000);
        assert_eq!(ranked[0].competition_discount, None);
    }

    #[test]
    fn source_reliability_discount_skips_non_commodity_goals() {
        let agent = entity(1);
        let view = base_view(agent);
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        assert_eq!(
            apply_source_reliability_discount(&goal(GoalKind::Sleep), &context, 100),
            None
        );
    }

    #[test]
    fn source_reliability_discount_returns_none_without_experience() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let mut view = base_view(agent);
        view.preference_profiles.insert(
            agent,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(1000),
                route_memory_capacity: 8,
                source_memory_capacity: 8,
                memory_retention_ticks: 100,
            },
        );
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        assert_eq!(
            apply_source_reliability_discount(
                &goal_at_place_with_sources(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                    },
                    market,
                    BTreeSet::from([source]),
                ),
                &context,
                100
            ),
            None
        );
    }

    #[test]
    fn source_reliability_discount_returns_none_without_preference_profile() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let mut view = base_view(agent);
        view.source_reliabilities.insert(
            agent,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(1, 1),
                )]),
            },
        );
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        assert_eq!(
            apply_source_reliability_discount(
                &goal_at_place_with_sources(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                    },
                    market,
                    BTreeSet::from([source]),
                ),
                &context,
                100
            ),
            None
        );
    }

    #[test]
    fn source_reliability_discount_applies_failure_ratio_proportionally() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let mut view = base_view(agent);
        view.preference_profiles.insert(
            agent,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(1000),
                route_memory_capacity: 8,
                source_memory_capacity: 8,
                memory_retention_ticks: 100,
            },
        );
        view.source_reliabilities.insert(
            agent,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(2, 2),
                )]),
            },
        );

        let ranked = rank(
            &[goal_at_place_with_sources(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                market,
                BTreeSet::from([source]),
            )],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 45_000);
        assert_eq!(
            ranked[0].source_reliability_discount,
            Some(SourceReliabilityDiscount {
                source_entity: source,
                commodity: CommodityKind::Bread,
                failure_ratio_permille: 500,
                pre_discount_motive: 90_000,
                post_discount_motive: 45_000,
            })
        );
    }

    #[test]
    fn source_reliability_discount_floors_positive_motive_at_one() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let mut view = base_view(agent);
        view.preference_profiles.insert(
            agent,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(1000),
                route_memory_capacity: 8,
                source_memory_capacity: 8,
                memory_retention_ticks: 100,
            },
        );
        view.source_reliabilities.insert(
            agent,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(0, 5),
                )]),
            },
        );
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        let discount = apply_source_reliability_discount(
            &goal_at_place_with_sources(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                market,
                BTreeSet::from([source]),
            ),
            &context,
            1,
        )
        .unwrap();

        assert_eq!(discount.failure_ratio_permille, 1000);
        assert_eq!(discount.pre_discount_motive, 1);
        assert_eq!(discount.post_discount_motive, 1);
    }

    #[test]
    fn source_reliability_discount_skips_zero_failure_ratio() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let mut view = base_view(agent);
        view.preference_profiles.insert(
            agent,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(1000),
                route_memory_capacity: 8,
                source_memory_capacity: 8,
                memory_retention_ticks: 100,
            },
        );
        view.source_reliabilities.insert(
            agent,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(3, 0),
                )]),
            },
        );
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        assert_eq!(
            apply_source_reliability_discount(
                &goal_at_place_with_sources(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                    },
                    market,
                    BTreeSet::from([source]),
                ),
                &context,
                100
            ),
            None
        );
    }

    #[test]
    fn source_reliability_discount_composes_with_competition_discount() {
        let agent = entity(1);
        let market = entity(2);
        let source = entity(50);
        let competitor = entity(10);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, entity(3));
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(entity(3)),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        view.preference_profiles.insert(
            agent,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(1000),
                route_memory_capacity: 8,
                source_memory_capacity: 8,
                memory_retention_ticks: 100,
            },
        );
        view.source_reliabilities.insert(
            agent,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(1, 1),
                )]),
            },
        );
        view.beliefs.insert(
            agent,
            vec![(
                competitor,
                observed_activity_state(market, ActionDomain::Production, None),
            )],
        );

        let ranked = rank(
            &[goal_at_place_with_sources(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                market,
                BTreeSet::from([source]),
            )],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 80_000);
        assert_eq!(
            ranked[0].source_reliability_discount,
            Some(SourceReliabilityDiscount {
                source_entity: source,
                commodity: CommodityKind::Bread,
                failure_ratio_permille: 500,
                pre_discount_motive: 200_000,
                post_discount_motive: 100_000,
            })
        );
        assert_eq!(
            ranked[0].competition_discount,
            Some(CompetitionDiscount {
                observed_competitors: vec![competitor],
                domain: ActionDomain::Production,
                effective_discount: pm(200),
                pre_discount_motive: 100_000,
                post_discount_motive: 80_000,
            })
        );
    }

    #[test]
    fn recipe_input_goals_inherit_downstream_self_care_priority_and_score() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );

        let recipe_id = teach_recipe(
            &mut view,
            agent,
            RecipeDefinition {
                name: "Bake Bread".to_string(),
                inputs: vec![(CommodityKind::Firewood, Quantity(1))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                work_ticks: NonZeroU32::new(3).unwrap(),
                required_workstation_tag: Some(WorkstationTag::Mill),
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::zero(),
            },
        );

        let ranked = rank(
            &[goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
    }

    #[test]
    fn move_cargo_scoring_uses_goal_commodity_directly() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank(
            &[goal(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: market,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn loot_candidates_are_low_and_suppressed_by_high_danger_or_self_care() {
        let agent = entity(1);
        let attacker = entity(9);
        let corpse = entity(3);
        let mut danger_view = base_view(agent);
        danger_view.attackers.insert(agent, vec![attacker]);

        let ranked = rank(
            &[goal(GoalKind::LootCorpse { corpse })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

        let mut self_care_view = base_view(agent);
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank(
            &[goal(GoalKind::LootCorpse { corpse })],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

        let ranked = rank(
            &[goal(GoalKind::BuryCorpse {
                corpse,
                burial_site: entity(4),
            })],
            &base_view(agent),
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(ranked[0].motive_score, 1);
    }

    #[test]
    fn share_belief_suppression_depends_on_communication_class() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let attacker = entity(9);
        let mut danger_view = base_view(agent);
        danger_view.attackers.insert(agent, vec![attacker]);
        danger_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );

        let ranked = rank(
            &[goal(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

        let ranked = rank(
            &[goal(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);

        let mut self_care_view = base_view(agent);
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );
        self_care_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );

        let ranked = rank(
            &[goal(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            })],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

        let mut critical_self_care_view = base_view(agent);
        critical_self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        critical_self_care_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );
        let ranked = rank(
            &[goal(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            })],
            &critical_self_care_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

        let mut calm_view = base_view(agent);
        calm_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );
        let ranked = rank(
            &[goal(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            })],
            &calm_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(
            ranked[0].motive_score,
            150 * u32::from(
                belief_confidence(
                    &PerceptionSource::DirectObservation,
                    1,
                    &BeliefConfidencePolicy::default(),
                )
                .value(),
            )
        );
    }

    #[test]
    fn alarm_share_belief_gets_a_saturating_motive_boost() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );

        let alarm_goal = goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Alarm,
        });
        let gossip_goal = goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        });

        let ranked = rank(
            &[alarm_goal, gossip_goal],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject },
                communication_class: worldwake_core::CommunicationClass::Alarm,
            }
        );
        assert!(
            ranked[0].motive_score > ranked[1].motive_score,
            "alarm-class ShareBelief should outrank equal-pressure gossip"
        );
        assert_eq!(
            ranked[0].motive_score,
            150 * u32::from(Permille::new_unchecked(1000).value())
        );
    }

    #[test]
    fn share_belief_scoring_tracks_social_weight_and_subject_belief_confidence() {
        let agent = entity(1);
        let listener = entity(2);
        let fresh_subject = entity(3);
        let rumor_subject = entity(4);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![
                (
                    fresh_subject,
                    believed_state(9, PerceptionSource::DirectObservation),
                ),
                (
                    rumor_subject,
                    believed_state(1, PerceptionSource::Rumor { chain_len: 3 }),
                ),
            ],
        );

        let baseline = utility();
        let stronger_social = UtilityProfile {
            social_weight: pm(300),
            ..baseline.clone()
        };
        let fresh_goal = goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief {
                subject: fresh_subject,
            },
            communication_class: worldwake_core::CommunicationClass::Testimony,
        });
        let rumor_goal = goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief {
                subject: rumor_subject,
            },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        });

        let baseline_ranked = rank(
            &[fresh_goal.clone(), rumor_goal.clone()],
            &view,
            agent,
            current_tick(),
            &baseline,
        )
        .into_ranked();
        let boosted_ranked = rank(
            &[fresh_goal, rumor_goal],
            &view,
            agent,
            current_tick(),
            &stronger_social,
        )
        .into_ranked();

        let fresh_pressure = belief_confidence(
            &PerceptionSource::DirectObservation,
            1,
            &view.belief_confidence_policy(agent),
        );
        let rumor_pressure = belief_confidence(
            &PerceptionSource::Rumor { chain_len: 3 },
            9,
            &view.belief_confidence_policy(agent),
        );

        assert!(baseline_ranked[0].motive_score > baseline_ranked[1].motive_score);
        assert_eq!(
            baseline_ranked[0].motive_score,
            150 * u32::from(fresh_pressure.value())
        );
        assert_eq!(
            baseline_ranked[1].motive_score,
            150 * u32::from(rumor_pressure.value())
        );
        assert_eq!(
            boosted_ranked[0].motive_score,
            300 * u32::from(fresh_pressure.value())
        );
    }

    #[test]
    fn share_belief_scoring_respects_per_agent_confidence_policy() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let mut skeptical_view = base_view(agent);
        skeptical_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(4, PerceptionSource::Rumor { chain_len: 2 }),
            )],
        );
        skeptical_view.confidence_policies.insert(
            agent,
            BeliefConfidencePolicy {
                rumor_base: pm(400),
                rumor_chain_penalty: pm(180),
                staleness_penalty_per_tick: pm(20),
                ..BeliefConfidencePolicy::default()
            },
        );

        let mut trusting_view = base_view(agent);
        trusting_view.beliefs = skeptical_view.beliefs.clone();
        trusting_view.confidence_policies.insert(
            agent,
            BeliefConfidencePolicy {
                rumor_base: pm(850),
                rumor_chain_penalty: pm(25),
                staleness_penalty_per_tick: pm(5),
                ..BeliefConfidencePolicy::default()
            },
        );

        let goal = goal(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        });
        let skeptical_ranked = rank(
            std::slice::from_ref(&goal),
            &skeptical_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        let trusting_ranked =
            rank(&[goal], &trusting_view, agent, current_tick(), &utility()).into_ranked();

        assert!(
            trusting_ranked[0].motive_score > skeptical_ranked[0].motive_score,
            "the acting agent's confidence policy should directly affect ShareBelief motive"
        );
    }

    #[test]
    fn share_belief_scoring_is_zero_without_social_weight_or_known_subject() {
        let agent = entity(1);
        let listener = entity(2);
        let known_subject = entity(3);
        let missing_subject = entity(4);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(
                known_subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );

        let zero_social = UtilityProfile {
            social_weight: pm(0),
            ..utility()
        };
        let ranked = rank(
            &[
                goal(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief {
                        subject: known_subject,
                    },
                    communication_class: worldwake_core::CommunicationClass::Testimony,
                }),
                goal(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief {
                        subject: missing_subject,
                    },
                    communication_class: worldwake_core::CommunicationClass::Gossip,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &zero_social,
        )
        .into_ranked();

        assert!(
            ranked.is_empty(),
            "zero social_weight and missing-subject goals should produce zero motive and be excluded from the ranked list"
        );
    }

    #[test]
    fn medium_priority_enterprise_and_critical_self_care_outrank_share_belief() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let market = entity(4);
        let facility = entity(5);
        let mut enterprise_view = base_view(agent);
        add_home_facility(&mut enterprise_view, market, facility);
        enterprise_view.beliefs.insert(
            agent,
            vec![(
                subject,
                believed_state(9, PerceptionSource::DirectObservation),
            )],
        );
        enterprise_view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        enterprise_view
            .demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let enterprise_first = rank(
            &[
                goal(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                    communication_class: worldwake_core::CommunicationClass::Testimony,
                }),
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &enterprise_view,
            agent,
            current_tick(),
            &UtilityProfile {
                enterprise_weight: pm(1),
                social_weight: pm(1000),
                ..utility()
            },
        )
        .into_ranked();
        assert!(matches!(
            enterprise_first[0].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread
            }
        ));

        let mut self_care_view = enterprise_view;
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        let self_care_first = rank(
            &[
                goal(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                    communication_class: worldwake_core::CommunicationClass::Testimony,
                }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert!(matches!(
            self_care_first[0].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            }
        ));
    }

    #[test]
    fn enterprise_does_not_outrank_critical_self_care() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank(
            &[
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert!(matches!(
            ranked.first().map(|goal| goal.grounded.key.kind),
            Some(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            })
        ));
    }

    #[test]
    fn same_priority_candidates_sort_by_motive_then_kind_then_ids() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let corpse_a = entity(10);
        let corpse_b = entity(11);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![
                demand(market, CommodityKind::Bread, 10),
                demand(market, CommodityKind::Water, 5),
            ],
        );

        let ranked = rank(
            &[
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Water,
                }),
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
                goal(GoalKind::LootCorpse { corpse: corpse_b }),
                goal(GoalKind::LootCorpse { corpse: corpse_a }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert!(matches!(
            ranked[0].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(
            ranked[2].grounded.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_a
        ));
        assert!(matches!(
            ranked[3].grounded.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_b
        ));
    }

    #[test]
    fn opportunity_signal_is_zero_without_demand_memory() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );

        let ranked = rank(
            &[goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert!(
            ranked.is_empty(),
            "restock goal with no demand memory should produce zero motive and be excluded from the ranked list"
        );
    }

    #[test]
    fn self_treat_wounds_uses_pain_weight_for_motive() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(650)]);

        let ranked = rank(
            &[goal(GoalKind::TreatWounds { patient: agent })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 400 * 650);
    }

    #[test]
    fn other_treat_wounds_uses_care_weight_for_motive() {
        let agent = entity(1);
        let patient = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(patient, vec![wound(650)]);

        let ranked = rank(
            &[goal(GoalKind::TreatWounds { patient })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 200 * 650);
    }

    #[test]
    fn high_care_weight_prioritizes_other_care_over_self_care() {
        let agent = entity(1);
        let patient = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(500)]);
        view.wounds.insert(patient, vec![wound(500)]);

        let profile = UtilityProfile {
            pain_weight: pm(100),
            care_weight: pm(900),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::TreatWounds { patient: agent }),
                goal(GoalKind::TreatWounds { patient }),
            ],
            &view,
            agent,
            current_tick(),
            &profile,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::TreatWounds { patient }
        );
        assert_eq!(ranked[0].motive_score, 900 * 500);
        assert_eq!(
            ranked[1].grounded.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[1].motive_score, 100 * 500);
    }

    #[test]
    fn high_pain_weight_prioritizes_self_care_over_other_care() {
        let agent = entity(1);
        let patient = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(500)]);
        view.wounds.insert(patient, vec![wound(500)]);

        let profile = UtilityProfile {
            pain_weight: pm(900),
            care_weight: pm(100),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::TreatWounds { patient: agent }),
                goal(GoalKind::TreatWounds { patient }),
            ],
            &view,
            agent,
            current_tick(),
            &profile,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].motive_score, 900 * 500);
        assert_eq!(
            ranked[1].grounded.key.kind,
            GoalKind::TreatWounds { patient }
        );
        assert_eq!(ranked[1].motive_score, 100 * 500);
    }

    #[test]
    fn produce_commodity_uses_recipe_outputs_for_opportunity_signal() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Firewood]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Firewood, 10)]);
        let recipe_id = teach_recipe(
            &mut view,
            agent,
            RecipeDefinition {
                name: "Cut Firewood".to_string(),
                inputs: vec![(CommodityKind::Grain, Quantity(2))],
                outputs: vec![(CommodityKind::Firewood, Quantity(1))],
                work_ticks: NonZeroU32::new(3).unwrap(),
                required_workstation_tag: None,
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(0), pm(1)),
            },
        );

        let ranked = rank(
            &[goal(GoalKind::ProduceCommodity { recipe_id })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn produce_commodity_uses_recipe_output_drive_when_recipe_serves_hunger() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(900), pm(100), pm(100), pm(100), pm(100)),
        );
        let recipe_id = teach_recipe(
            &mut view,
            agent,
            RecipeDefinition {
                name: "Bake Bread".to_string(),
                inputs: vec![(CommodityKind::Firewood, Quantity(1))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                work_ticks: NonZeroU32::new(3).unwrap(),
                required_workstation_tag: Some(WorkstationTag::Mill),
                required_tool_kinds: Vec::new(),
                body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(0), pm(1)),
            },
        );

        let ranked = rank(
            &[goal(GoalKind::ProduceCommodity { recipe_id })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
        assert!(matches!(
            ranked[0].provenance,
            Some(RankedGoalProvenance::Drive(_))
        ));
    }

    #[test]
    fn ranking_is_deterministic_for_identical_inputs() {
        let agent = entity(1);
        let market = entity(2);
        let facility = entity(3);
        let mut view = base_view(agent);
        add_home_facility(&mut view, market, facility);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        let candidates = vec![
            goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            goal(GoalKind::LootCorpse { corpse: entity(5) }),
            goal(GoalKind::Sleep),
        ];

        let first = rank(&candidates, &view, agent, current_tick(), &utility()).into_ranked();
        let second = rank(&candidates, &view, agent, current_tick(), &utility()).into_ranked();

        assert_eq!(first, second);
    }

    #[test]
    fn simultaneous_critical_self_care_needs_rank_by_weighted_order() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                thresholds.hunger.critical(),
                thresholds.thirst.critical(),
                thresholds.fatigue.critical(),
                pm(0),
                pm(0),
            ),
        );
        let utility = UtilityProfile {
            hunger_weight: pm(800),
            thirst_weight: pm(600),
            fatigue_weight: pm(400),
            ..UtilityProfile::default()
        };

        let ranked = rank(
            &[
                goal(GoalKind::Sleep),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Water,
                }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility,
        )
        .into_ranked();

        assert!(matches!(
            ranked[0].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(ranked[2].grounded.key.kind, GoalKind::Sleep));
    }

    #[test]
    fn clotted_wound_boosts_hunger_high_to_critical() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        match ranked[0]
            .provenance
            .as_ref()
            .expect("hunger candidate should carry drive provenance")
        {
            RankedGoalProvenance::Drive(provenance) => {
                assert_eq!(provenance.base_priority_class, GoalPriorityClass::High);
                assert_eq!(provenance.final_priority_class, GoalPriorityClass::Critical);
                assert_eq!(
                    provenance.adjustment,
                    Some(RankedPriorityAdjustment::ClottedWoundRecoveryPromotion)
                );
                assert_eq!(provenance.motive_inputs.len(), 1);
                assert_eq!(provenance.motive_inputs[0].drive, RankedDriveKind::Hunger);
                assert_eq!(
                    provenance.motive_inputs[0].pressure,
                    thresholds.hunger.high()
                );
                assert_eq!(provenance.motive_inputs[0].weight, utility().hunger_weight);
                assert_eq!(
                    provenance.motive_inputs[0].score,
                    u32::from(utility().hunger_weight.value())
                        * u32::from(thresholds.hunger.high().value())
                );
                assert!(provenance.motive_inputs[0].recovery_relevant);
            }
            RankedGoalProvenance::Danger(_) => {
                panic!("hunger candidate should not use danger provenance")
            }
        }
    }

    #[test]
    fn bleeding_wound_no_boost() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.wounds.insert(agent, vec![wound_with_bleed(200, 10)]);

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::High);
    }

    #[test]
    fn clotted_wound_no_boost_below_high() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        let below_high = thresholds.hunger.high().saturating_sub(pm(1));
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(below_high, pm(0), pm(0), pm(0), pm(0)),
        );
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_ne!(ranked[0].priority_class, GoalPriorityClass::Critical);
    }

    #[test]
    fn clotted_wound_boosts_sleep_high_to_critical() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), thresholds.fatigue.high(), pm(0), pm(0)),
        );
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::Sleep)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
    }

    #[test]
    fn clotted_wound_no_boost_relieve_or_wash() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                pm(0),
                pm(0),
                pm(0),
                thresholds.bladder.high(),
                thresholds.dirtiness.high(),
            ),
        );
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::Relieve), goal(GoalKind::Wash)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::High);
        assert_eq!(ranked[1].priority_class, GoalPriorityClass::High);
        for goal in &ranked {
            match goal
                .provenance
                .as_ref()
                .expect("drive goals should carry drive provenance")
            {
                RankedGoalProvenance::Drive(provenance) => {
                    assert_eq!(provenance.base_priority_class, GoalPriorityClass::High);
                    assert_eq!(provenance.final_priority_class, GoalPriorityClass::High);
                    assert_eq!(provenance.adjustment, None);
                    assert_eq!(provenance.motive_inputs.len(), 1);
                    assert!(!provenance.motive_inputs[0].recovery_relevant);
                }
                RankedGoalProvenance::Danger(_) => {
                    panic!("relieve/wash should not use danger provenance")
                }
            }
        }
    }

    #[test]
    fn promoted_hunger_outranks_higher_motive_wash_when_clotted_wound_recovery_applies() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(760), pm(0), pm(0), pm(0), pm(860)),
        );
        view.wounds.insert(agent, vec![wound(200)]);
        let utility = UtilityProfile::default();

        let ranked = rank(
            &[
                goal(GoalKind::Wash),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility,
        )
        .into_ranked();

        assert_eq!(ranked.len(), 2);
        let bread = &ranked[0];
        let wash = &ranked[1];

        assert_eq!(
            bread.grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }
        );
        assert_eq!(bread.priority_class, GoalPriorityClass::Critical);
        assert_eq!(bread.motive_score, 380_000);

        assert_eq!(wash.grounded.key.kind, GoalKind::Wash);
        assert_eq!(wash.priority_class, GoalPriorityClass::High);
        assert_eq!(wash.motive_score, 430_000);
        assert!(wash.motive_score > bread.motive_score);

        match bread
            .provenance
            .as_ref()
            .expect("bread goal should carry drive provenance")
        {
            RankedGoalProvenance::Drive(provenance) => {
                assert_eq!(provenance.base_priority_class, GoalPriorityClass::High);
                assert_eq!(provenance.final_priority_class, GoalPriorityClass::Critical);
                assert_eq!(
                    provenance.adjustment,
                    Some(RankedPriorityAdjustment::ClottedWoundRecoveryPromotion)
                );
                assert_eq!(provenance.motive_inputs.len(), 1);
                assert_eq!(provenance.motive_inputs[0].drive, RankedDriveKind::Hunger);
                assert_eq!(provenance.motive_inputs[0].pressure, pm(760));
                assert_eq!(provenance.motive_inputs[0].weight, utility.hunger_weight);
                assert_eq!(provenance.motive_inputs[0].score, 380_000);
                assert!(provenance.motive_inputs[0].recovery_relevant);
            }
            RankedGoalProvenance::Danger(_) => {
                panic!("bread goal should not use danger provenance")
            }
        }

        match wash
            .provenance
            .as_ref()
            .expect("wash goal should carry drive provenance")
        {
            RankedGoalProvenance::Drive(provenance) => {
                assert_eq!(provenance.base_priority_class, GoalPriorityClass::High);
                assert_eq!(provenance.final_priority_class, GoalPriorityClass::High);
                assert_eq!(provenance.adjustment, None);
                assert_eq!(provenance.motive_inputs.len(), 1);
                assert_eq!(
                    provenance.motive_inputs[0].drive,
                    RankedDriveKind::Dirtiness
                );
                assert_eq!(provenance.motive_inputs[0].pressure, pm(860));
                assert_eq!(provenance.motive_inputs[0].weight, utility.dirtiness_weight);
                assert_eq!(provenance.motive_inputs[0].score, 430_000);
                assert!(!provenance.motive_inputs[0].recovery_relevant);
            }
            RankedGoalProvenance::Danger(_) => {
                panic!("wash goal should not use danger provenance")
            }
        }
    }

    #[test]
    fn no_wounds_no_boost() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::High);
    }

    #[test]
    fn critical_stays_critical() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
    }

    #[test]
    fn claim_office_uses_enterprise_weight_and_medium_priority() {
        let agent = entity(1);
        let view = base_view(agent);

        let ranked = rank(
            &[goal(GoalKind::ClaimOffice { office: entity(7) })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(
            ranked[0].motive_score,
            u32::from(utility().enterprise_weight.value())
        );
    }

    #[test]
    fn raid_target_scores_from_known_loot_opportunity() {
        let agent = entity(1);
        let target = entity(7);
        let mut view = base_view(agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.alive.insert(target);
        view.place_entities
            .entry(view.effective_places[&agent])
            .or_default()
            .push(target);
        view.effective_places
            .insert(target, view.effective_places[&agent]);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );
        view.commodity_quantities
            .insert((target, CommodityKind::Apple), Quantity(4));

        let utility = UtilityProfile {
            danger_weight: pm(999),
            enterprise_weight: pm(0),
            ..utility()
        };

        let ranked = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility,
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            ranked[0].motive_score,
            4 * u32::from(utility.hunger_weight.value()) * 700
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].provenance, None);
    }

    #[test]
    fn raid_target_is_zero_motive_without_known_loot() {
        let agent = entity(1);
        let target = entity(7);
        let mut view = base_view(agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.alive.insert(target);
        view.place_entities
            .entry(view.effective_places[&agent])
            .or_default()
            .push(target);
        view.effective_places
            .insert(target, view.effective_places[&agent]);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(ranked.into_ranked().is_empty());
    }

    #[test]
    fn raid_target_is_zero_motive_when_wound_deterrence_is_active() {
        let agent = entity(1);
        let target = entity(7);
        let faction = entity(8);
        let mut view = base_view(agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.alive.insert(target);
        view.place_entities
            .entry(view.effective_places[&agent])
            .or_default()
            .push(target);
        view.effective_places
            .insert(target, view.effective_places[&agent]);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );
        view.commodity_quantities
            .insert((target, CommodityKind::Apple), Quantity(4));
        view.courage.insert(agent, pm(200));
        view.wounds.insert(agent, vec![wound(120), wound(120)]);
        view.factions_by_member.insert(agent, vec![faction]);
        view.bandit_flee_thresholds.insert(faction, pm(300));

        let ranked = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(ranked.into_ranked().is_empty());

        view.wounds.insert(agent, vec![wound(120), wound(100)]);
        let ranked = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
    }

    #[test]
    fn regroup_with_faction_uses_medium_priority_and_social_weight() {
        let agent = entity(1);
        let faction = entity(7);
        let view = base_view(agent);
        let utility = UtilityProfile {
            social_weight: pm(420),
            enterprise_weight: pm(999),
            ..utility()
        };

        let ranked = rank(
            &[goal(GoalKind::RegroupWithFaction { faction })],
            &view,
            agent,
            current_tick(),
            &utility,
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(
            ranked[0].motive_score,
            u32::from(utility.social_weight.value())
        );
        assert!(ranked[0].provenance.is_none());
    }

    #[test]
    fn establish_bandit_camp_uses_full_social_pressure_and_outranks_share_belief() {
        let agent = entity(1);
        let faction = entity(7);
        let listener = entity(8);
        let subject = entity(9);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(
                subject,
                worldwake_core::BelievedEntityState {
                    last_known_place: Some(entity(10)),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: current_tick(),
                    source: PerceptionSource::DirectObservation,
                },
            )],
        );
        let utility = UtilityProfile {
            social_weight: pm(420),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::EntityBelief { subject },
                    communication_class: worldwake_core::CommunicationClass::Testimony,
                }),
                goal(GoalKind::EstablishBanditCamp { faction }),
            ],
            &view,
            agent,
            current_tick(),
            &utility,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::EstablishBanditCamp { faction }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 420 * 1000);
    }

    #[test]
    fn critical_self_treat_outranks_claim_office_even_with_lower_motive() {
        let agent = entity(1);
        let office = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(850)]);
        let profile = UtilityProfile {
            pain_weight: pm(1),
            enterprise_weight: pm(1000),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::TreatWounds { patient: agent }),
                goal(GoalKind::ClaimOffice { office }),
            ],
            &view,
            agent,
            current_tick(),
            &profile,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 850);
        assert_eq!(
            ranked[1].grounded.key.kind,
            GoalKind::ClaimOffice { office }
        );
        assert_eq!(ranked[1].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[1].motive_score, 1000);
    }

    #[test]
    fn medium_self_treat_and_claim_office_tie_break_on_motive() {
        let agent = entity(1);
        let office = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(350)]);
        let profile = UtilityProfile {
            pain_weight: pm(3),
            enterprise_weight: pm(1000),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::TreatWounds { patient: agent }),
                goal(GoalKind::ClaimOffice { office }),
            ],
            &view,
            agent,
            current_tick(),
            &profile,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 1050);
        assert_eq!(
            ranked[1].grounded.key.kind,
            GoalKind::ClaimOffice { office }
        );
        assert_eq!(ranked[1].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[1].motive_score, 1000);
    }

    #[test]
    fn low_self_treat_ranks_below_claim_office() {
        let agent = entity(1);
        let office = entity(7);
        let mut view = base_view(agent);
        view.wounds.insert(agent, vec![wound(200)]);
        let profile = UtilityProfile {
            pain_weight: pm(1000),
            enterprise_weight: pm(1),
            ..utility()
        };

        let ranked = rank(
            &[
                goal(GoalKind::TreatWounds { patient: agent }),
                goal(GoalKind::ClaimOffice { office }),
            ],
            &view,
            agent,
            current_tick(),
            &profile,
        )
        .into_ranked();

        assert_eq!(
            ranked[0].grounded.key.kind,
            GoalKind::ClaimOffice { office }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 1);
        assert_eq!(
            ranked[1].grounded.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[1].priority_class, GoalPriorityClass::Low);
        assert_eq!(ranked[1].motive_score, 200_000);
    }

    #[test]
    fn support_candidate_uses_social_weight_times_loyalty() {
        let agent = entity(1);
        let candidate = entity(2);
        let mut view = base_view(agent);
        view.loyalties.insert((agent, candidate), pm(600));

        let ranked = rank(
            &[goal(GoalKind::SupportCandidateForOffice {
                office: entity(7),
                candidate,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(
            ranked[0].motive_score,
            u32::from(utility().social_weight.value()) * u32::from(pm(600).value())
        );
    }

    // ── Feasibility sort-order tests ──

    fn make_ranked_goal(
        kind: GoalKind,
        priority_class: GoalPriorityClass,
        motive: u32,
        feasibility: crate::feasibility::FeasibilityHint,
    ) -> crate::RankedGoal {
        crate::RankedGoal {
            grounded: GroundedGoal {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey {
                    kind,
                    commodity: None,
                    entity: None,
                    place: None,
                },
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class,
            motive_score: motive,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility,
        }
    }

    #[test]
    fn test_feasibility_tiebreak_within_priority_class() {
        use crate::feasibility::FeasibilityHint;
        let mut goals = [
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::Critical,
                900,
                FeasibilityHint::Unlikely,
            ),
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::Critical,
                600,
                FeasibilityHint::Likely,
            ),
        ];
        goals.sort_by(super::compare_ranked_goals);
        // Likely(600) should outrank Unlikely(900) within same priority class.
        assert_eq!(goals[0].feasibility, FeasibilityHint::Likely);
        assert_eq!(goals[0].motive_score, 600);
        assert_eq!(goals[1].feasibility, FeasibilityHint::Unlikely);
        assert_eq!(goals[1].motive_score, 900);
    }

    #[test]
    fn test_feasibility_does_not_cross_priority_class() {
        use crate::feasibility::FeasibilityHint;
        let mut goals = [
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::Low,
                500,
                FeasibilityHint::Likely,
            ),
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::Critical,
                500,
                FeasibilityHint::Unlikely,
            ),
        ];
        goals.sort_by(super::compare_ranked_goals);
        // Critical+Unlikely must still outrank Low+Likely.
        assert_eq!(goals[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(goals[1].priority_class, GoalPriorityClass::Low);
    }

    #[test]
    fn test_same_feasibility_falls_through_to_motive() {
        use crate::feasibility::FeasibilityHint;
        let mut goals = [
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::High,
                400,
                FeasibilityHint::Uncertain,
            ),
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::High,
                800,
                FeasibilityHint::Uncertain,
            ),
        ];
        goals.sort_by(super::compare_ranked_goals);
        // Same priority class + same feasibility → higher motive wins.
        assert_eq!(goals[0].motive_score, 800);
        assert_eq!(goals[1].motive_score, 400);
    }

    #[test]
    fn explain_ranked_goal_order_reports_decisive_dimension() {
        use crate::feasibility::FeasibilityHint;

        let winner = make_ranked_goal(
            GoalKind::Sleep,
            GoalPriorityClass::High,
            800,
            FeasibilityHint::Likely,
        );
        let loser = make_ranked_goal(
            GoalKind::Wash,
            GoalPriorityClass::High,
            600,
            FeasibilityHint::Likely,
        );

        let comparison =
            super::explain_ranked_goal_order(&winner, &loser).expect("ordering should explain");

        assert_eq!(comparison.winner.goal_key.kind, GoalKind::Sleep);
        assert_eq!(comparison.loser.goal_key.kind, GoalKind::Wash);
        assert_eq!(
            comparison.decisive_dimension,
            super::RankedGoalComparisonDimension::MotiveScore
        );
    }

    #[test]
    fn fulfill_bounty_uses_enterprise_weighted_reward_motive() {
        let agent = entity(1);
        let bounty = entity(2);
        let issuer = entity(3);
        let target = entity(4);
        let claim_place = entity(5);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(
                bounty,
                believed_bounty_state(
                    issuer,
                    claim_place,
                    BountyTarget::EliminateEntity { target },
                    250,
                ),
            )],
        );

        let ranked = rank(
            &[goal(GoalKind::FulfillBounty { bounty })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 50_000);
    }

    #[test]
    fn fulfill_delivery_bounty_uses_same_enterprise_weighted_reward_motive() {
        let agent = entity(1);
        let bounty = entity(2);
        let issuer = entity(3);
        let claim_place = entity(5);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(
                bounty,
                believed_bounty_state(
                    issuer,
                    claim_place,
                    BountyTarget::DeliverCommodity {
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(3),
                        destination: claim_place,
                    },
                    250,
                ),
            )],
        );

        let ranked = rank(
            &[goal(GoalKind::FulfillBounty { bounty })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked[0].motive_score, 50_000);
    }

    #[test]
    fn explain_ranked_goal_order_reports_priority_class_for_political_over_social() {
        use crate::feasibility::FeasibilityHint;

        let office = entity(7);
        let listener = entity(8);
        let political = make_ranked_goal(
            GoalKind::ClaimOffice { office },
            GoalPriorityClass::Medium,
            200,
            FeasibilityHint::Uncertain,
        );
        let social = make_ranked_goal(
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                        office,
                        holder: None,
                        effective_tick: Tick(1),
                    },
                },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            },
            GoalPriorityClass::Low,
            950_000,
            FeasibilityHint::Likely,
        );

        let comparison =
            super::explain_ranked_goal_order(&political, &social).expect("ordering should explain");

        assert_eq!(
            comparison.winner.goal_key.kind,
            GoalKind::ClaimOffice { office }
        );
        assert_eq!(
            comparison.loser.goal_key.kind,
            GoalKind::ShareBelief {
                listener,
                topic: TellTopic::InstitutionalClaim {
                    claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                        office,
                        holder: None,
                        effective_tick: Tick(1),
                    },
                },
                communication_class: worldwake_core::CommunicationClass::Testimony,
            }
        );
        assert_eq!(
            comparison.decisive_dimension,
            super::RankedGoalComparisonDimension::PriorityClass
        );
    }
}
