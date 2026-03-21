use crate::{
    assess_danger, classify_band, derive_danger_pressure, derive_pain_pressure,
    enterprise::{market_signal_for_place, opportunity_signal},
    evaluate_suppression, DecisionContext, GoalPolicyOutcome, GoalPriorityClass, GroundedGoal,
    RankedDriveGoalProvenance, RankedDriveKind, RankedDriveMotiveInput, RankedGoal,
    RankedGoalProvenance, RankedPriorityAdjustment,
};
use std::cmp::Ordering;
use worldwake_core::{
    belief_confidence, BelievedEntityState, CommodityKind, CommodityPurpose, DriveThresholds,
    EntityId, GoalKey, GoalKind, HomeostaticNeeds, Permille, ThresholdBand, Tick,
    UtilityProfile,
};
use worldwake_sim::{GoalBeliefView, RecipeRegistry};

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
    recipes: &RecipeRegistry,
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
        let provenance = goal_ranking_provenance(candidate, &context, recipes);
        let scored = RankedGoal {
            grounded: candidate.clone(),
            priority_class: ranked_priority_class(candidate, &context, recipes, provenance.as_ref()),
            motive_score: ranked_motive_score(candidate, &context, recipes, provenance.as_ref()),
            provenance,
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
    recipes: &RecipeRegistry,
    provenance: Option<&RankedGoalProvenance>,
) -> GoalPriorityClass {
    provenance.cloned().map_or_else(
        || priority_class(candidate, context, recipes),
        |provenance| match provenance {
            RankedGoalProvenance::Danger(_) => context.decision_context.danger_class,
            RankedGoalProvenance::Drive(provenance) => provenance.final_priority_class,
        },
    )
}

fn ranked_motive_score(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
    provenance: Option<&RankedGoalProvenance>,
) -> u32 {
    provenance.cloned().map_or_else(
        || motive_score(candidate, context, recipes),
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

fn goal_ranking_provenance(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> Option<RankedGoalProvenance> {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => self_consume_provenance(commodity, context).map(RankedGoalProvenance::Drive),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        }
        | GoalKind::ProduceCommodity { recipe_id } => {
            recipe_output_provenance(recipe_id, context, recipes).map(RankedGoalProvenance::Drive)
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
        GoalKind::EngageHostile { .. } | GoalKind::ReduceDanger => {
            Some(RankedGoalProvenance::Danger(context.danger_assessment.clone()))
        }
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
}

fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent)
        .into_iter()
        .any(|wound| wound.severity.value() > 0 && wound.bleed_rate_per_tick.value() == 0)
}

fn priority_class(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> GoalPriorityClass {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => self_consume_priority(commodity, context),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        } => recipe_output_priority(recipe_id, context, recipes),
        GoalKind::AcquireCommodity { .. }
        | GoalKind::ProduceCommodity { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::MoveCargo { .. }
        | GoalKind::ClaimOffice { .. } => GoalPriorityClass::Medium,
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
        | GoalKind::ShareBelief { .. }
        | GoalKind::SupportCandidateForOffice { .. } => GoalPriorityClass::Low,
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
    let final_priority_class = promote_for_clotted_wound_recovery(
        base_priority_class,
        context,
        recovery_relevant,
    );
    RankedDriveGoalProvenance {
        base_priority_class,
        final_priority_class,
        adjustment: (final_priority_class != base_priority_class)
            .then_some(RankedPriorityAdjustment::ClottedWoundRecoveryPromotion),
        motive_inputs,
    }
}

fn motive_score(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> u32 {
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
        } => recipe_output_motive_score(recipe_id, context, recipes),
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
        GoalKind::TreatWounds { patient } => {
            let patient_pain = derive_pain_pressure(context.view, patient);
            if patient == context.agent {
                score_product(context.utility.pain_weight, patient_pain)
            } else {
                score_product(context.utility.care_weight, patient_pain)
            }
        }
        GoalKind::ProduceCommodity { recipe_id } => {
            let signal = recipes
                .get(recipe_id)
                .map_or(Permille::new_unchecked(0), |recipe| {
                    recipe
                        .outputs
                        .iter()
                        .map(|(commodity, _)| {
                            opportunity_signal(
                                context.view,
                                context.agent,
                                context.view.effective_place(context.agent),
                                *commodity,
                            )
                        })
                        .max_by_key(|signal| signal.value())
                        .unwrap_or(Permille::new_unchecked(0))
                });
            score_product(context.utility.enterprise_weight, signal)
        }
        GoalKind::MoveCargo {
            commodity,
            destination,
        } => {
            let signal =
                market_signal_for_place(context.view, context.agent, commodity, destination);
            score_product(context.utility.enterprise_weight, signal)
        }
        GoalKind::ShareBelief { subject, .. } => score_product(
            context.utility.social_weight,
            social_pressure_for_subject(context, subject),
        ),
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } => 1,
        GoalKind::ClaimOffice { .. } => u32::from(context.utility.enterprise_weight.value()),
        GoalKind::SupportCandidateForOffice { candidate, .. } => context
            .view
            .loyalty_to(context.agent, candidate)
            .map_or(0, |loyalty| {
                score_product(context.utility.social_weight, loyalty)
            }),
    }
}

fn social_pressure_for_subject(context: &RankingContext<'_>, subject: EntityId) -> Permille {
    let belief = context
        .view
        .known_entity_beliefs(context.agent)
        .into_iter()
        .find_map(|(entity, belief)| (entity == subject).then_some(belief));

    belief.map_or(Permille::new_unchecked(0), |belief| {
        belief_pressure_from_state(
            &belief,
            context.current_tick,
            &context.view.belief_confidence_policy(context.agent),
        )
    })
}

fn belief_pressure_from_state(
    state: &BelievedEntityState,
    current_tick: Tick,
    policy: &worldwake_core::BeliefConfidencePolicy,
) -> Permille {
    let staleness_ticks = current_tick.0.saturating_sub(state.observed_tick.0);
    belief_confidence(&state.source, staleness_ticks, policy)
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

fn recipe_output_priority(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> GoalPriorityClass {
    recipes
        .get(recipe_id)
        .map_or(GoalPriorityClass::Background, |recipe| {
            recipe
                .outputs
                .iter()
                .map(|(commodity, _)| commodity_goal_priority(*commodity, context))
                .max()
                .unwrap_or(GoalPriorityClass::Background)
        })
}

fn commodity_goal_priority(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> GoalPriorityClass {
    let self_consume = self_consume_priority(commodity, context);
    if self_consume > GoalPriorityClass::Background {
        return self_consume;
    }

    if enterprise_score(commodity, context) > 0 {
        GoalPriorityClass::Medium
    } else {
        GoalPriorityClass::Background
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
            recovery_relevant: factor.recovery_relevant,
        })
        .collect::<Vec<_>>();
    (!motive_inputs.is_empty()).then(|| {
        drive_provenance_from_inputs(context, base_priority_class, motive_inputs)
    })
}

fn recipe_output_provenance(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> Option<RankedDriveGoalProvenance> {
    recipes.get(recipe_id).and_then(|recipe| {
        recipe
            .outputs
            .iter()
            .filter_map(|(commodity, _)| self_consume_provenance(*commodity, context))
            .max_by_key(|provenance| {
                provenance
                    .motive_inputs
                    .iter()
                    .map(|input| input.score)
                    .max()
                    .unwrap_or(0)
            })
    })
}

fn recipe_output_motive_score(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> u32 {
    recipes.get(recipe_id).map_or(0, |recipe| {
        recipe
            .outputs
            .iter()
            .map(|(commodity, _)| commodity_goal_motive_score(*commodity, context))
            .max()
            .unwrap_or(0)
    })
}

fn commodity_goal_motive_score(commodity: CommodityKind, context: &RankingContext<'_>) -> u32 {
    let self_consume = relevant_self_consume_factors(commodity, context)
        .into_iter()
        .map(|factor| score_product(factor.weight, factor.pressure))
        .max()
        .unwrap_or(0);
    if self_consume > 0 {
        return self_consume;
    }

    enterprise_score(commodity, context)
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
        });
    }
    if profile.thirst_relief_per_unit.value() > 0 {
        factors.push(DriveFactor {
            drive: RankedDriveKind::Thirst,
            pressure: needs.thirst,
            weight: context.utility.thirst_weight,
            band: thresholds.thirst,
            recovery_relevant: true,
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

fn score_product(weight: Permille, pressure: Permille) -> u32 {
    u32::from(weight.value()) * u32::from(pressure.value())
}

fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
    right
        .priority_class
        .cmp(&left.priority_class)
        .then_with(|| right.motive_score.cmp(&left.motive_score))
        .then_with(|| {
            goal_kind_discriminant(left.grounded.key.kind)
                .cmp(&goal_kind_discriminant(right.grounded.key.kind))
        })
        .then_with(|| {
            left.grounded
                .key
                .commodity
                .cmp(&right.grounded.key.commodity)
        })
        .then_with(|| left.grounded.key.entity.cmp(&right.grounded.key.entity))
        .then_with(|| left.grounded.key.place.cmp(&right.grounded.key.place))
}

fn goal_kind_discriminant(kind: GoalKind) -> u8 {
    match kind {
        GoalKind::ConsumeOwnedCommodity { .. } => 0,
        GoalKind::AcquireCommodity { .. } => 1,
        GoalKind::Sleep => 2,
        GoalKind::Relieve => 3,
        GoalKind::Wash => 4,
        GoalKind::EngageHostile { .. } => 5,
        GoalKind::ReduceDanger => 6,
        GoalKind::TreatWounds { .. } => 7,
        GoalKind::ProduceCommodity { .. } => 8,
        GoalKind::SellCommodity { .. } => 9,
        GoalKind::RestockCommodity { .. } => 10,
        GoalKind::MoveCargo { .. } => 11,
        GoalKind::LootCorpse { .. } => 12,
        GoalKind::BuryCorpse { .. } => 13,
        GoalKind::ShareBelief { .. } => 14,
        GoalKind::ClaimOffice { .. } => 15,
        GoalKind::SupportCandidateForOffice { .. } => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_decision_context, rank_candidates};
    use crate::{
        GoalKey, GoalKind, GoalPriorityClass, GroundedGoal, RankedDriveKind,
        RankedGoalProvenance, RankedPriorityAdjustment,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        belief_confidence, BeliefConfidencePolicy, BelievedEntityState, BodyCostPerTick, BodyPart,
        CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DemandObservation, DemandObservationReason, DeprivationKind, DriveThresholds, EntityId,
        EntityKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile,
        MetabolismProfile, PerceptionSource, Permille, Quantity, RecipeId, ResourceSource, Tick,
        TickRange, TradeDispositionProfile, UniqueItemKind, UtilityProfile, WorkstationTag, Wound,
        WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, DurationExpr, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView,
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        confidence_policies: BTreeMap<EntityId, BeliefConfidencePolicy>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        loyalties: BTreeMap<(EntityId, EntityId), Permille>,
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
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
        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
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
        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
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
        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
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
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
        );
        view.thresholds.insert(agent, DriveThresholds::default());
        view.confidence_policies
            .insert(agent, BeliefConfidencePolicy::default());
        view
    }

    /// Test helper: builds `DecisionContext` from the view and delegates to `rank_candidates`.
    fn rank(
        candidates: &[GroundedGoal],
        view: &TestBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &UtilityProfile,
        recipes: &RecipeRegistry,
    ) -> super::RankingOutcome {
        let dc = build_decision_context(view, agent);
        rank_candidates(candidates, view, agent, current_tick, utility, recipes, &dc)
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
            &RecipeRegistry::new(),
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
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
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
            &RecipeRegistry::new(),
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
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

        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });

        let ranked = rank(
            &[goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &recipes,
        )
        .into_ranked();

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
    }

    #[test]
    fn move_cargo_scoring_uses_goal_commodity_directly() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
        )
        .into_ranked();
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(ranked[0].motive_score, 1);
    }

    #[test]
    fn share_belief_is_low_priority_and_suppressed_by_high_danger_or_self_care() {
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
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        )
        .into_ranked();
        assert!(ranked.is_empty());

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
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
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
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &calm_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
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
            subject: fresh_subject,
        });
        let rumor_goal = goal(GoalKind::ShareBelief {
            listener,
            subject: rumor_subject,
        });

        let baseline_ranked = rank(
            &[fresh_goal.clone(), rumor_goal.clone()],
            &view,
            agent,
            current_tick(),
            &baseline,
            &RecipeRegistry::new(),
        )
        .into_ranked();
        let boosted_ranked = rank(
            &[fresh_goal, rumor_goal],
            &view,
            agent,
            current_tick(),
            &stronger_social,
            &RecipeRegistry::new(),
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

        let goal = goal(GoalKind::ShareBelief { listener, subject });
        let skeptical_ranked = rank(
            std::slice::from_ref(&goal),
            &skeptical_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        )
        .into_ranked();
        let trusting_ranked = rank(
            &[goal],
            &trusting_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        )
        .into_ranked();

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
                    subject: known_subject,
                }),
                goal(GoalKind::ShareBelief {
                    listener,
                    subject: missing_subject,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &zero_social,
            &RecipeRegistry::new(),
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
        let mut enterprise_view = base_view(agent);
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
                home_market: Some(market),
            },
        );
        enterprise_view
            .demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let enterprise_first = rank(
            &[
                goal(GoalKind::ShareBelief { listener, subject }),
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
            &RecipeRegistry::new(),
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
                goal(GoalKind::ShareBelief { listener, subject }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
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
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
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
            &RecipeRegistry::new(),
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
        let corpse_a = entity(10);
        let corpse_b = entity(11);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
                home_market: Some(market),
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
            &RecipeRegistry::new(),
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
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Firewood]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Firewood, 10)]);
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Cut Firewood".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Firewood, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1)),
        });

        let ranked = rank(
            &[goal(GoalKind::ProduceCommodity { recipe_id })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &recipes,
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
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Firewood, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1)),
        });

        let ranked = rank(
            &[goal(GoalKind::ProduceCommodity { recipe_id })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &recipes,
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
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
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

        let first = rank(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        )
        .into_ranked();
        let second = rank(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        )
        .into_ranked();

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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
                assert_eq!(provenance.motive_inputs[0].pressure, thresholds.hunger.high());
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
            &RecipeRegistry::new(),
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
        view.needs
            .insert(agent, HomeostaticNeeds::new(below_high, pm(0), pm(0), pm(0), pm(0)));
        view.wounds.insert(agent, vec![wound(200)]);

        let ranked = rank(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
                assert_eq!(provenance.motive_inputs[0].drive, RankedDriveKind::Dirtiness);
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
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
            &RecipeRegistry::new(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(
            ranked[0].motive_score,
            u32::from(utility().social_weight.value()) * u32::from(pm(600).value())
        );
    }
}
