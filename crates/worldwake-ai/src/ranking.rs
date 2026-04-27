//! # Preference-ordering authority
//!
//! `ranking::compare_ranked_goals` is the sole authoritative total order on
//! `AgendaEntry`. It is file-private and therefore unreachable from outside
//! this module.
//!
//! ```compile_fail
//! use worldwake_ai::ranking::compare_ranked_goals;
//! ```
//!
//! `OrderedRanked::from_sorted_for_test` is the in-crate test escape hatch
//! and is not reachable from outside `worldwake-ai`.
//!
//! ```compile_fail
//! use worldwake_ai::ranking::OrderedRanked;
//! use worldwake_ai::AgendaEntry;
//!
//! let empty: &[AgendaEntry] = &[];
//! let _ = OrderedRanked::from_sorted_for_test(empty);
//! ```

use crate::{
    AgendaEntry, DecisionContext, GoalKindPlannerExt, GoalOffer, GoalPolicyOutcome,
    GoalPriorityClass, OpportunityExpectationFailureIncident, RankedDriveGoalProvenance,
    RankedDriveKind, RankedDriveMotiveInput, RankedGoalProvenance, RankedGoalProvenanceFamily,
    RankedPriorityAdjustment, assess_danger, classify_band,
    decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    derive_danger_pressure, derive_pain_pressure,
    enterprise::{market_signal_for_place, opportunity_signal},
    evaluate_suppression,
    goal_model::free_carry_capacity_contract_from_view,
    pressure::is_bandit_raid_deterred_by_wounds,
    route_threat::threat_warning_signal_for_place,
    theft::assess_theft_deterrence,
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use worldwake_core::{
    ActionDomain, BelievedEntityState, BountyTarget, CommodityKind, CommodityPurpose,
    CommunicationClass, DeprivationExposure, DiversificationProfile, DriveEscalationProfile,
    DriveThresholds, EntityId, ExpectationBasis, ExpectationOutcome, ExpectationRecord,
    ExpectationState, ExplorationMotivation, ExplorationProfile, GoalKey, GoalKind,
    GoalRejectionReason, HomeostaticNeedId, HomeostaticNeeds, InstitutionalBeliefRead,
    InstitutionalClaim, InstitutionalKnowledgeSource, LearnedOpportunityMemory, MultiplierPermille,
    NoticeTopic, ObligationExecutionTracker, ObligationSatiationProfile, OpportunityAnchor,
    OpportunityKey, PerceptionSource, Permille, Quantity, ReliabilityRecord, RepairKey,
    RepairMemory, RightKind, SourceKey, SubstitutePreferences, TellTopic, ThresholdBand, Tick,
    UtilityProfile, ViolationKind, belief_confidence, escalation_multiplier,
    failure_ratio_permille,
};
use worldwake_sim::{CommodityOpportunityBreakdown, GoalBeliefView, commodity_opportunity_score};

/// Outcome of the ranking pipeline, preserving information about filtered candidates.
#[derive(Clone, Debug)]
pub struct RankingOutcome {
    /// Ranked goals after all filters (sorted by ranking order).
    pub(crate) ranked: Vec<AgendaEntry>,
    /// Goals that were suppressed by situational conditions (danger/self-care pressure).
    pub(crate) suppressed: Vec<crate::candidate_generation::CandidateSuppressionDiagnostic>,
    /// Goals that passed suppression but had zero motive score.
    pub zero_motive: Vec<GoalKey>,
}

/// A read-only view over `AgendaEntry`s ordered by the authoritative preference
/// defined in `ranking::compare_ranked_goals`.
#[derive(Clone, Copy, Debug)]
pub struct OrderedRanked<'a> {
    slice: &'a [AgendaEntry],
}

impl<'a> OrderedRanked<'a> {
    fn new(slice: &'a [AgendaEntry]) -> Self {
        Self { slice }
    }

    #[cfg(test)]
    pub(crate) fn from_sorted_for_test(slice: &'a [AgendaEntry]) -> Self {
        Self { slice }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    #[must_use]
    pub fn first(&self) -> Option<&AgendaEntry> {
        self.slice.first()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, AgendaEntry> {
        self.slice.iter()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[AgendaEntry] {
        self.slice
    }

    pub fn find(&self, pred: impl Fn(&AgendaEntry) -> bool) -> Option<&AgendaEntry> {
        self.slice.iter().find(|goal| pred(goal))
    }
}

impl<'b> IntoIterator for &'b OrderedRanked<'_> {
    type Item = &'b AgendaEntry;
    type IntoIter = std::slice::Iter<'b, AgendaEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl RankingOutcome {
    /// Consume the outcome, returning only the ranked goals.
    #[must_use]
    pub fn into_ranked(self) -> Vec<AgendaEntry> {
        self.ranked
    }

    /// Borrow the outcome's ranked goals as an ordered read-only view.
    #[must_use]
    pub fn ordered(&self) -> OrderedRanked<'_> {
        OrderedRanked::new(self.ranked.as_slice())
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
    candidates: &[GoalOffer],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    decision_context: &DecisionContext,
) -> RankingOutcome {
    rank_candidates_with_memories(
        candidates,
        view,
        agent,
        current_tick,
        utility,
        *decision_context,
        &RepairMemory::default(),
        &LearnedOpportunityMemory::default(),
    )
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn rank_candidates_with_memories(
    candidates: &[GoalOffer],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    decision_context: DecisionContext,
    repair_memory: &RepairMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
) -> RankingOutcome {
    let context = RankingContext::with_memories(
        view,
        agent,
        current_tick,
        utility,
        decision_context,
        repair_memory,
        learned_opportunity_memory,
    );

    let mut suppressed = Vec::new();
    let mut zero_motive = Vec::new();

    let mut ranked = Vec::new();
    for candidate in candidates {
        if !matches!(
            evaluate_suppression(&candidate.key.kind, &context.decision_context),
            GoalPolicyOutcome::Available
        ) {
            suppressed.push(
                crate::candidate_generation::CandidateSuppressionDiagnostic {
                    opportunity: OpportunityKey {
                        goal_key: candidate.key,
                        anchor: candidate.anchor,
                    },
                    reason: GoalRejectionReason::SuppressedByStressPolicy,
                },
            );
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
        let scored = AgendaEntry::pending(
            candidate.clone(),
            current_tick,
            priority_class,
            competition_discount
                .as_ref()
                .map_or(post_source_reliability_motive, |discount| {
                    discount.post_discount_motive
                }),
            provenance,
            source_reliability_discount,
            competition_discount,
            crate::feasibility::FeasibilityHint::Uncertain,
        );
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

/// Sort a `Vec<AgendaEntry>` by authoritative preference and return a view
/// borrowing the sorted storage.
#[must_use]
pub fn sort_in_place(ranked: &mut Vec<AgendaEntry>) -> OrderedRanked<'_> {
    ranked.sort_unstable_by(compare_ranked_goals);
    OrderedRanked::new(ranked.as_slice())
}

fn ranked_priority_class(
    candidate: &GoalOffer,
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
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    provenance: Option<&RankedGoalProvenance>,
) -> u32 {
    let base = provenance.cloned().map_or_else(
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
    );
    base.saturating_add(memory_motive_bonus(candidate, context, base))
}

fn memory_motive_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> u32 {
    if base_motive == 0 {
        return 0;
    }

    repair_memory_bonus(candidate, context, base_motive).saturating_add(learned_opportunity_bonus(
        candidate,
        context,
        base_motive,
    ))
}

fn repair_memory_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> u32 {
    let alternate_target = match candidate.anchor {
        OpportunityAnchor::Place(place) | OpportunityAnchor::Entity(place) => place,
        OpportunityAnchor::None => return 0,
    };
    let repair_key = RepairKey {
        goal_key: candidate.key,
        alternate_target,
    };
    let Some(entry) = context.repair_memory.repairs.get(&repair_key) else {
        return 0;
    };
    if entry.expires_tick <= context.current_tick {
        return 0;
    }

    (base_motive / 10)
        .max(1)
        .saturating_mul(entry.success_count.max(1))
}

fn learned_opportunity_bonus(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    base_motive: u32,
) -> u32 {
    let opportunity = OpportunityKey {
        goal_key: candidate.key,
        anchor: candidate.anchor,
    };
    let Some(entry) = context
        .learned_opportunity_memory
        .opportunities
        .get(&opportunity)
    else {
        return 0;
    };
    if entry.expires_tick <= context.current_tick {
        return 0;
    }

    (base_motive / 20).max(1)
}

fn apply_competition_discount(
    candidate: &GoalOffer,
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
    candidate: &GoalOffer,
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

pub(crate) fn apply_pending_source_reliability_failures(
    ranked: &mut Vec<AgendaEntry>,
    inputs: &PendingSourceReliabilityInputs<'_>,
    pending_failures: &[OpportunityExpectationFailureIncident],
) {
    if pending_failures.is_empty() {
        return;
    }
    let pending_failure_sources = pending_failures
        .iter()
        .map(|incident| incident.source)
        .collect::<Vec<_>>();

    let context = RankingContext::with_memories(
        inputs.view,
        inputs.agent,
        inputs.current_tick,
        inputs.utility,
        inputs.decision_context,
        inputs.repair_memory,
        inputs.learned_opportunity_memory,
    );

    for entry in ranked.iter_mut() {
        let Some((source_entity, commodity)) = source_reliability_discount_scope(&entry.offer)
        else {
            continue;
        };
        let source_key = SourceKey {
            entity: source_entity,
            commodity,
        };
        if !pending_failure_sources.contains(&source_key) {
            continue;
        }

        let pre_source_motive = entry.source_reliability_discount.as_ref().map_or_else(
            || {
                entry
                    .competition_discount
                    .as_ref()
                    .map_or(entry.motive_score, |discount| discount.pre_discount_motive)
            },
            |discount| discount.pre_discount_motive,
        );
        let source_reliability_discount = apply_source_reliability_discount_with_pending_failures(
            &entry.offer,
            &context,
            pre_source_motive,
            &pending_failure_sources,
        );
        let post_source_motive = source_reliability_discount
            .as_ref()
            .map_or(pre_source_motive, |discount| discount.post_discount_motive);
        let competition_discount =
            apply_competition_discount(&entry.offer, &context, post_source_motive);

        entry.source_reliability_discount = source_reliability_discount;
        entry.competition_discount = competition_discount;
        entry.motive_score = entry
            .competition_discount
            .as_ref()
            .map_or(post_source_motive, |discount| discount.post_discount_motive);
    }

    let _ = sort_in_place(ranked);
}

pub(crate) struct PendingSourceReliabilityInputs<'a> {
    pub(crate) view: &'a dyn GoalBeliefView,
    pub(crate) agent: EntityId,
    pub(crate) current_tick: Tick,
    pub(crate) utility: &'a UtilityProfile,
    pub(crate) decision_context: DecisionContext,
    pub(crate) repair_memory: &'a RepairMemory,
    pub(crate) learned_opportunity_memory: &'a LearnedOpportunityMemory,
}

fn apply_source_reliability_discount_with_pending_failures(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    motive_score: u32,
    pending_failures: &[SourceKey],
) -> Option<SourceReliabilityDiscount> {
    if motive_score == 0 {
        return None;
    }

    let (source_entity, commodity) = source_reliability_discount_scope(candidate)?;
    let source_key = SourceKey {
        entity: source_entity,
        commodity,
    };
    if !pending_failures.contains(&source_key) {
        return apply_source_reliability_discount(candidate, context, motive_score);
    }

    let profile = context.view.preference_profile(context.agent)?;
    let trust_weight = u32::from(profile.source_trust_weight.value());
    let mut record = context
        .view
        .source_reliability(context.agent)
        .and_then(|source_reliability| source_reliability.sources.get(&source_key).copied())
        .unwrap_or(ReliabilityRecord {
            successful_acquisitions: 0,
            failed_attempts: 0,
            last_attempt_tick: context.current_tick,
        });
    record.failed_attempts = record.failed_attempts.saturating_add(1);
    let failure_ratio = failure_ratio_permille(&record);
    if failure_ratio == 0 {
        return None;
    }

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

fn source_reliability_discount_scope(candidate: &GoalOffer) -> Option<(EntityId, CommodityKind)> {
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

fn competition_discount_scope(candidate: &GoalOffer) -> Option<(ActionDomain, EntityId)> {
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
    candidate: &GoalOffer,
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
            quantity: _,
        } => self_consume_provenance(*commodity, context).map(RankedGoalProvenance::Drive),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
            quantity: _,
        }
        | GoalKind::ProduceCommodity { recipe_id } => {
            best_recipe_output_assessment(*recipe_id, context)
                .and_then(|assessment| assessment.provenance)
                .map(RankedGoalProvenance::Drive)
        }
        GoalKind::LootCorpse { corpse } => corpse_loot_assessment(*corpse, context)
            .and_then(|assessment| assessment.provenance)
            .map(RankedGoalProvenance::Drive),
        GoalKind::StealItem { target_item } => steal_item_assessment(*target_item, context)
            .and_then(|assessment| assessment.provenance)
            .map(RankedGoalProvenance::Drive),
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
    repair_memory: &'a RepairMemory,
    learned_opportunity_memory: &'a LearnedOpportunityMemory,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    exposure: Option<DeprivationExposure>,
    escalation_profile: Option<DriveEscalationProfile>,
    exploration_profile: Option<ExplorationProfile>,
    diversification_profile: Option<DiversificationProfile>,
    last_proactive_exploration_tick: Option<Tick>,
    satiation_profile: ObligationSatiationProfile,
    obligation_tracker: ObligationExecutionTracker,
    has_clotted_wounds: bool,
    danger_assessment: crate::DangerAssessment,
    danger_pressure: Permille,
    decision_context: DecisionContext,
    holdings: BTreeMap<CommodityKind, u32>,
    local_alternatives: BTreeMap<CommodityKind, u32>,
}

impl<'a> RankingContext<'a> {
    #[cfg_attr(not(test), allow(dead_code))]
    fn new(
        view: &'a dyn GoalBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &'a UtilityProfile,
        decision_context: DecisionContext,
    ) -> Self {
        Self::with_memories(
            view,
            agent,
            current_tick,
            utility,
            decision_context,
            empty_repair_memory(),
            empty_learned_opportunity_memory(),
        )
    }

    fn with_memories(
        view: &'a dyn GoalBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &'a UtilityProfile,
        decision_context: DecisionContext,
        repair_memory: &'a RepairMemory,
        learned_opportunity_memory: &'a LearnedOpportunityMemory,
    ) -> Self {
        let danger_assessment = assess_danger(view, agent);
        let satiation_profile = view.obligation_satiation_profile(agent);
        let mut obligation_tracker = view.obligation_execution_tracker(agent);
        let window_start = current_tick
            .0
            .saturating_sub(u64::from(satiation_profile.window_ticks));
        obligation_tracker
            .completion_ticks
            .retain(|tick| tick.0 >= window_start);
        Self {
            view,
            agent,
            current_tick,
            utility,
            repair_memory,
            learned_opportunity_memory,
            needs: view.homeostatic_needs(agent),
            thresholds: view.drive_thresholds(agent),
            exposure: view.deprivation_exposure(agent),
            escalation_profile: view.drive_escalation_profile(agent),
            exploration_profile: view.exploration_profile(agent),
            diversification_profile: view.diversification_profile(agent),
            last_proactive_exploration_tick: view.last_proactive_exploration_tick(agent),
            satiation_profile,
            obligation_tracker,
            has_clotted_wounds: has_clotted_wounds(view, agent),
            danger_pressure: danger_assessment.pressure,
            danger_assessment,
            decision_context,
            holdings: holdings_from_view(view, agent),
            local_alternatives: local_alternatives_from_view(view, agent),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn empty_repair_memory() -> &'static RepairMemory {
    static EMPTY: OnceLock<RepairMemory> = OnceLock::new();
    EMPTY.get_or_init(RepairMemory::default)
}

#[cfg_attr(not(test), allow(dead_code))]
fn empty_learned_opportunity_memory() -> &'static LearnedOpportunityMemory {
    static EMPTY: OnceLock<LearnedOpportunityMemory> = OnceLock::new();
    EMPTY.get_or_init(LearnedOpportunityMemory::default)
}

#[derive(Copy, Clone)]
struct DriveFactor {
    drive: RankedDriveKind,
    pressure: Permille,
    weight: Permille,
    escalation_multiplier: MultiplierPermille,
    band: ThresholdBand,
    recovery_relevant: bool,
    relief_per_unit: Permille,
}

fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent)
        .into_iter()
        .any(|wound| wound.severity.value() > 0 && wound.bleed_rate_per_tick.value() == 0)
}

fn priority_class(candidate: &GoalOffer, context: &RankingContext<'_>) -> GoalPriorityClass {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
            quantity: _,
        } => self_consume_priority(commodity, context),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
            quantity: _,
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
        GoalKind::LootCorpse { corpse } => corpse_loot_assessment(corpse, context)
            .map_or(GoalPriorityClass::Low, |assessment| {
                assessment.priority_class
            }),
        GoalKind::StealItem { target_item } => steal_item_assessment(target_item, context)
            .map_or(GoalPriorityClass::Low, |assessment| {
                assessment.priority_class
            }),
        GoalKind::ExploreLocation {
            motivating_need: ExplorationMotivation::NeedDriven(need_id),
            ..
        } => drive_priority(
            context,
            |needs| need_pressure_for_id(needs, need_id),
            |thresholds| threshold_band_for_need(thresholds, need_id),
            need_driven_exploration_recovery_relevant(need_id),
        ),
        GoalKind::FreeCarryCapacity
        | GoalKind::BuryCorpse { .. }
        | GoalKind::SearchForMissing { .. }
        | GoalKind::ReportMissing { .. }
        | GoalKind::ReportFound { .. }
        | GoalKind::EscortToSafety { .. }
        | GoalKind::ShareBelief { .. }
        | GoalKind::InvestigateViolation { .. }
        | GoalKind::Patrol { .. }
        | GoalKind::ExploreLocation {
            motivating_need: ExplorationMotivation::Proactive,
            ..
        }
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
        vec![ranked_drive_motive_input(
            context,
            drive,
            pressure,
            weight,
            Permille::new_unchecked(1000),
            recovery_relevant,
        )],
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
        commodity_preference_rank: None,
        motive_inputs,
    }
}

/// Tiebreak bonus added to `AcquireCommodity` motive scores so agents whose
/// goal demands more units rank the goal slightly higher when other things
/// are equal. Capped at `+100` to keep the base urgency-driven score
/// dominant. Single-unit acquisition (existing baseline) gets `+0`, so
/// pre-quantity-aware ranking semantics are preserved for `AcquisitionQuantity::single()`.
fn acquire_commodity_quantity_bonus(quantity: worldwake_core::AcquisitionQuantity) -> u32 {
    u32::from(quantity.desired_target.get().saturating_sub(1)).min(100)
}

fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity } => {
            relevant_self_consume_factors(commodity, context)
                .into_iter()
                .map(effective_drive_factor_score)
                .max()
                .unwrap_or(0)
        }
        GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
            quantity,
        } => {
            let base = relevant_self_consume_factors(commodity, context)
                .into_iter()
                .map(effective_drive_factor_score)
                .max()
                .unwrap_or(0);
            base.saturating_add(acquire_commodity_quantity_bonus(quantity))
        }
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
            quantity,
        } => best_recipe_output_assessment(recipe_id, context)
            .map_or(0, |assessment| assessment.motive_score)
            .saturating_add(acquire_commodity_quantity_bonus(quantity)),
        GoalKind::ProduceCommodity { recipe_id } => {
            best_recipe_output_assessment(recipe_id, context)
                .map_or(0, |assessment| assessment.motive_score)
        }
        GoalKind::AcquireCommodity {
            commodity,
            quantity,
            ..
        } => enterprise_score(commodity, context)
            .saturating_add(acquire_commodity_quantity_bonus(quantity)),
        GoalKind::SellCommodity { commodity } | GoalKind::RestockCommodity { commodity } => {
            enterprise_score(commodity, context)
        }
        GoalKind::Sleep => drive_score(
            context,
            HomeostaticNeedId::Fatigue,
            |needs| needs.fatigue,
            |utility| utility.fatigue_weight,
        ),
        GoalKind::Relieve => drive_score(
            context,
            HomeostaticNeedId::Bladder,
            |needs| needs.bladder,
            |utility| utility.bladder_weight,
        ),
        GoalKind::Wash => drive_score(
            context,
            HomeostaticNeedId::Dirtiness,
            |needs| needs.dirtiness,
            |utility| utility.dirtiness_weight,
        ),
        GoalKind::FreeCarryCapacity => free_carry_capacity_contract_from_view(
            context.view,
            context.agent,
        )
        .map_or(0, |contract| {
            if !contract.is_actionable() {
                return 0;
            }
            let strain = Permille::new_unchecked(
                ((contract.current_load.0 * 1000) / contract.carry_capacity.0.max(1)).min(1000)
                    as u16,
            );
            score_product(context.utility.enterprise_weight, strain)
        }),
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
        GoalKind::LootCorpse { corpse } => corpse_loot_assessment(corpse, context)
            .map_or(1, |assessment| assessment.motive_score.max(1)),
        GoalKind::BuryCorpse { .. } => 1,
        GoalKind::SearchForMissing { .. }
        | GoalKind::ReportMissing { .. }
        | GoalKind::ReportFound { .. } => expectation_response_motive(&candidate.key.kind, context),
        GoalKind::EscortToSafety { subject, .. } => {
            // Escort motive is lower than TreatWounds so that agents prefer
            // healing a co-located wounded entity over escorting them away.
            // Quarter the score to ensure TreatWounds always outranks escort
            // when both compete for the same patient.
            let subject_pain = derive_pain_pressure(context.view, subject);
            score_product(context.utility.care_weight, subject_pain) / 4
        }
        GoalKind::Patrol { .. } => patrol_motive(context),
        GoalKind::ExploreLocation {
            motivating_need, ..
        } => exploration_motive(context, motivating_need),
        GoalKind::StealItem { target_item } => steal_item_assessment(target_item, context)
            .map_or_else(
                || theft_motive(context),
                |assessment| assessment.motive_score.max(1),
            ),
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

fn exploration_motive(context: &RankingContext<'_>, motivating_need: ExplorationMotivation) -> u32 {
    let Some(needs) = context.needs else {
        return 0;
    };
    match motivating_need {
        ExplorationMotivation::NeedDriven(need_id) => {
            let Some(profile) = context.exploration_profile else {
                return 0;
            };
            drive_score(
                context,
                need_id,
                |needs| need_pressure_for_id(needs, need_id),
                |utility| utility_weight_for_need(utility, need_id),
            )
            .saturating_mul(u32::from(profile.curiosity_weight.value()))
                / 1000
        }
        ExplorationMotivation::Proactive => {
            let Some(profile) = context.diversification_profile else {
                return 0;
            };
            let max_need = needs.max_value();
            if max_need > profile.comfort_threshold.value() {
                return 0;
            }
            let need_slack = Permille::new_unchecked(1000u16.saturating_sub(max_need));
            let curiosity_pressure = proactive_curiosity_pressure(
                context.current_tick,
                context.last_proactive_exploration_tick,
                profile,
            );
            u32::from(profile.base_curiosity.value())
                .saturating_mul(u32::from(curiosity_pressure.value()))
                .saturating_mul(u32::from(need_slack.value()))
                / 1_000_000
        }
    }
}

fn proactive_curiosity_pressure(
    current_tick: Tick,
    last_proactive_tick: Option<Tick>,
    profile: DiversificationProfile,
) -> Permille {
    let ticks_since = last_proactive_tick
        .map_or(current_tick.0, |tick| current_tick.0.saturating_sub(tick.0))
        .min(1000);
    let raw = ticks_since.saturating_mul(u64::from(profile.curiosity_buildup_rate.value()));
    Permille::new_unchecked(raw.min(1000) as u16)
}

fn utility_weight_for_need(utility: &UtilityProfile, need_id: HomeostaticNeedId) -> Permille {
    match need_id {
        HomeostaticNeedId::Hunger => utility.hunger_weight,
        HomeostaticNeedId::Thirst => utility.thirst_weight,
        HomeostaticNeedId::Fatigue => utility.fatigue_weight,
        HomeostaticNeedId::Bladder => utility.bladder_weight,
        HomeostaticNeedId::Dirtiness => utility.dirtiness_weight,
    }
}

fn threshold_band_for_need(
    thresholds: DriveThresholds,
    need_id: HomeostaticNeedId,
) -> worldwake_core::ThresholdBand {
    match need_id {
        HomeostaticNeedId::Hunger => thresholds.hunger,
        HomeostaticNeedId::Thirst => thresholds.thirst,
        HomeostaticNeedId::Fatigue => thresholds.fatigue,
        HomeostaticNeedId::Bladder => thresholds.bladder,
        HomeostaticNeedId::Dirtiness => thresholds.dirtiness,
    }
}

fn need_driven_exploration_recovery_relevant(need_id: HomeostaticNeedId) -> bool {
    matches!(
        need_id,
        HomeostaticNeedId::Hunger | HomeostaticNeedId::Thirst
    )
}

fn need_pressure_for_id(needs: HomeostaticNeeds, need_id: HomeostaticNeedId) -> Permille {
    match need_id {
        HomeostaticNeedId::Hunger => needs.hunger,
        HomeostaticNeedId::Thirst => needs.thirst,
        HomeostaticNeedId::Fatigue => needs.fatigue,
        HomeostaticNeedId::Bladder => needs.bladder,
        HomeostaticNeedId::Dirtiness => needs.dirtiness,
    }
}

fn expectation_response_motive(goal_kind: &GoalKind, context: &RankingContext<'_>) -> u32 {
    let signal = expectation_response_signal(goal_kind, context);
    if signal == 0 {
        return 0;
    }

    let weight = match goal_kind {
        GoalKind::SearchForMissing { .. } => context.utility.care_weight,
        GoalKind::ReportMissing { .. } | GoalKind::ReportFound { .. } => {
            context.utility.social_weight
        }
        _ => return 0,
    };

    u32::from(weight.value()).saturating_mul(signal)
}

fn expectation_response_signal(goal_kind: &GoalKind, context: &RankingContext<'_>) -> u32 {
    let subject = match goal_kind {
        GoalKind::SearchForMissing { subject, .. }
        | GoalKind::ReportMissing { subject, .. }
        | GoalKind::ReportFound { subject, .. } => *subject,
        _ => return 0,
    };

    let Some(store) = context.view.expectation_store(context.agent) else {
        return 0;
    };

    let is_report_found = matches!(goal_kind, GoalKind::ReportFound { .. });
    store
        .records
        .values()
        .filter(|record| {
            record.owner == context.agent
                && record.subject == subject
                && if is_report_found {
                    matches!(
                        record.state,
                        ExpectationState::Resolved {
                            outcome: ExpectationOutcome::FoundSafe { .. }
                                | ExpectationOutcome::FoundWounded { .. }
                                | ExpectationOutcome::FoundDead { .. }
                        }
                    )
                } else {
                    record.state == ExpectationState::Overdue
                }
        })
        .map(|record| expectation_signal_from_record(*record, context.current_tick))
        .max()
        .unwrap_or(0)
}

fn expectation_signal_from_record(record: ExpectationRecord, current_tick: Tick) -> u32 {
    let overdue = current_tick
        .0
        .saturating_sub(record.deadline_tick.0.saturating_add(record.grace_ticks))
        .max(1);
    let overdue = u32::try_from(overdue).unwrap_or(u32::MAX);
    let basis_weight = u32::from(expectation_basis_weight(record.basis));
    basis_weight.saturating_mul(overdue)
}

fn expectation_basis_weight(basis: ExpectationBasis) -> u8 {
    match basis {
        ExpectationBasis::DutyAssignment { .. } | ExpectationBasis::EscortObligation { .. } => 3,
        ExpectationBasis::DeliveryCommitment { .. } => 2,
        ExpectationBasis::RoutineReturn | ExpectationBasis::SocialPromise => 1,
        ExpectationBasis::PlanStepCompletion { .. } => 0, // plan-step expectations are agent-internal; no social-obligation weight
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
                        | worldwake_core::InstitutionalClaim::Verdict { effective_tick, .. }
                        | worldwake_core::InstitutionalClaim::MissingPersonStatus {
                            effective_tick,
                            ..
                        } => effective_tick.0,
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

    let raw_score = score_product(
        context.utility.bounty_posting_weight,
        reward_signal_from_quantity(terms.reward_quantity),
    );
    apply_obligation_satiation(context, raw_score)
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
    let raw_score = score_product(context.utility.notice_posting_weight, threat_signal);
    apply_obligation_satiation(context, raw_score)
}

fn apply_obligation_satiation(context: &RankingContext<'_>, raw_score: u32) -> u32 {
    let recent_count = context.obligation_tracker.completion_ticks.len() as u32;
    if recent_count <= context.satiation_profile.satiation_threshold {
        return raw_score;
    }

    let over_threshold = recent_count - context.satiation_profile.satiation_threshold;
    let decay_total = over_threshold.saturating_mul(u32::from(
        context.satiation_profile.decay_per_execution.value(),
    ));
    let multiplier = 1000u32
        .saturating_sub(decay_total)
        .max(u32::from(context.satiation_profile.satiation_floor.value()));
    raw_score.saturating_mul(multiplier) / 1000
}

fn belief_pressure_from_state(
    state: &BelievedEntityState,
    current_tick: Tick,
    policy: &worldwake_core::BeliefConfidencePolicy,
) -> Permille {
    belief_pressure_from_source(
        state.source,
        state.last_observed_tick().unwrap_or(Tick(0)),
        current_tick,
        policy,
    )
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

fn investigation_motive(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
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
    need: HomeostaticNeedId,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    weight: impl Fn(&UtilityProfile) -> Permille,
) -> u32 {
    match context.needs {
        Some(needs) => effective_motive_score(
            score_product(weight(context.utility), pressure(needs)),
            drive_escalation_multiplier(context, need),
        ),
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
        .map(|factor| {
            ranked_drive_motive_input(
                context,
                factor.drive,
                factor.pressure,
                factor.weight,
                factor.relief_per_unit,
                factor.recovery_relevant,
            )
        })
        .collect::<Vec<_>>();
    (!motive_inputs.is_empty()).then(|| {
        let mut provenance =
            drive_provenance_from_inputs(context, base_priority_class, motive_inputs);
        provenance.commodity_preference_rank = substitute_preference_rank(
            context.view.substitute_preferences(context.agent),
            commodity,
        );
        provenance
    })
}

fn substitute_preference_rank(
    preferences: Option<SubstitutePreferences>,
    commodity: CommodityKind,
) -> Option<u8> {
    let category = commodity.spec().trade_category;
    preferences?
        .preferences
        .get(&category)?
        .iter()
        .position(|preferred| *preferred == commodity)
        .and_then(|rank| u8::try_from(rank).ok())
}

fn raid_target_motive(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    let GoalKind::RaidTarget { target } = candidate.key.kind else {
        unreachable!("raid_target_motive requires RaidTarget");
    };

    if is_bandit_raid_deterred_by_wounds(context.view, context.agent) {
        return 0;
    }

    let base = CommodityKind::ALL
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
        .sum();

    scale_motive_by_confidence(
        base,
        context
            .view
            .believed_target_location(context.agent, target)
            .confidence,
    )
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
            escalation_multiplier: drive_escalation_multiplier(context, HomeostaticNeedId::Hunger),
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
            escalation_multiplier: drive_escalation_multiplier(context, HomeostaticNeedId::Thirst),
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

fn effective_motive_score(base_score: u32, multiplier: MultiplierPermille) -> u32 {
    base_score.saturating_mul(u32::from(multiplier.value())) / 1000
}

fn scale_motive_by_confidence(base_score: u32, confidence: Permille) -> u32 {
    base_score.saturating_mul(u32::from(confidence.value())) / 1000
}

fn drive_escalation_multiplier(
    context: &RankingContext<'_>,
    need: HomeostaticNeedId,
) -> MultiplierPermille {
    let ticks = context
        .exposure
        .map_or(0, |exposure| exposure.ticks_at_critical(need));
    let params = context
        .escalation_profile
        .as_ref()
        .map(|profile| profile.params_for(need))
        .unwrap_or_default();
    escalation_multiplier(ticks, params)
}

fn effective_drive_factor_score(factor: DriveFactor) -> u32 {
    effective_motive_score(
        score_product(factor.weight, factor.pressure),
        factor.escalation_multiplier,
    )
}

fn ranked_drive_motive_input(
    context: &RankingContext<'_>,
    drive: RankedDriveKind,
    pressure: Permille,
    weight: Permille,
    relief_per_unit: Permille,
    recovery_relevant: bool,
) -> RankedDriveMotiveInput {
    RankedDriveMotiveInput {
        drive,
        pressure,
        weight,
        score: score_product(weight, pressure),
        escalation_multiplier: drive_escalation_multiplier(
            context,
            homeostatic_need_id_for_drive(drive),
        ),
        relief_per_unit,
        recovery_relevant,
    }
}

fn homeostatic_need_id_for_drive(kind: RankedDriveKind) -> HomeostaticNeedId {
    match kind {
        RankedDriveKind::Hunger => HomeostaticNeedId::Hunger,
        RankedDriveKind::Thirst => HomeostaticNeedId::Thirst,
        RankedDriveKind::Fatigue => HomeostaticNeedId::Fatigue,
        RankedDriveKind::Bladder => HomeostaticNeedId::Bladder,
        RankedDriveKind::Dirtiness => HomeostaticNeedId::Dirtiness,
    }
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
            .map(effective_drive_factor_score)
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

#[derive(Clone, Debug)]
struct CorpseLootAssessment {
    priority_class: GoalPriorityClass,
    motive_score: u32,
    provenance: Option<RankedDriveGoalProvenance>,
}

#[derive(Clone, Debug)]
struct StealItemAssessment {
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

fn corpse_loot_assessment(
    corpse: EntityId,
    context: &RankingContext<'_>,
) -> Option<CorpseLootAssessment> {
    let direct_lot_quantities = context
        .view
        .direct_possessions(corpse)
        .into_iter()
        .filter_map(|entity| {
            let commodity = context.view.item_lot_commodity(entity)?;
            let quantity = context.view.commodity_quantity(entity, commodity);
            (quantity > Quantity(0)).then_some((commodity, quantity))
        });
    let direct_corpse_quantities = CommodityKind::ALL.iter().copied().filter_map(|commodity| {
        let quantity = context.view.commodity_quantity(corpse, commodity);
        (quantity > Quantity(0)).then_some((commodity, quantity))
    });

    direct_lot_quantities
        .chain(direct_corpse_quantities)
        .filter_map(|(commodity, quantity)| {
            Some({
                let mut simulated_holdings = context.holdings.clone();
                *simulated_holdings.entry(commodity).or_insert(0) += quantity.0;
                let breakdown = commodity_opportunity_score(
                    context.agent,
                    commodity,
                    context.view,
                    &simulated_holdings,
                    &context.local_alternatives,
                );
                let direct_survival = breakdown.direct_survival_score > 0;
                let treatment = breakdown.treatment_score > 0;
                (direct_survival || treatment).then_some(CorpseLootAssessment {
                    priority_class: commodity_shared_priority(commodity, breakdown, context),
                    motive_score: commodity_shared_motive_score(commodity, breakdown, context),
                    provenance: commodity_shared_provenance(commodity, breakdown, context),
                })
            })?
        })
        .max_by(|left, right| {
            left.priority_class
                .cmp(&right.priority_class)
                .then_with(|| left.motive_score.cmp(&right.motive_score))
        })
}

fn theft_motive_scale(context: &RankingContext<'_>) -> Permille {
    assess_theft_deterrence(context.view, context.agent).map_or(Permille::ZERO, |assessment| {
        Permille::new(assessment.effective_motive.min(1000) as u16)
            .unwrap_or(Permille::new_unchecked(1000))
    })
}

fn scale_drive_provenance_motive(
    mut provenance: RankedDriveGoalProvenance,
    scale: Permille,
) -> RankedDriveGoalProvenance {
    for input in &mut provenance.motive_inputs {
        input.score = scale_motive_by_confidence(input.score, scale);
    }
    provenance
}

fn steal_item_assessment(
    target_item: EntityId,
    context: &RankingContext<'_>,
) -> Option<StealItemAssessment> {
    let commodity = context.view.item_lot_commodity(target_item)?;
    let quantity = context.view.commodity_quantity(target_item, commodity);
    if quantity == Quantity(0) {
        return None;
    }

    let mut simulated_holdings = context.holdings.clone();
    *simulated_holdings.entry(commodity).or_insert(0) += quantity.0;
    let breakdown = commodity_opportunity_score(
        context.agent,
        commodity,
        context.view,
        &simulated_holdings,
        &context.local_alternatives,
    );
    let direct_survival = breakdown.direct_survival_score > 0;
    let treatment = breakdown.treatment_score > 0;
    if !(direct_survival || treatment) {
        return None;
    }
    let motive_scale = theft_motive_scale(context);

    Some(StealItemAssessment {
        priority_class: commodity_shared_priority(commodity, breakdown, context),
        motive_score: scale_motive_by_confidence(
            commodity_shared_motive_score(commodity, breakdown, context),
            motive_scale,
        ),
        provenance: commodity_shared_provenance(commodity, breakdown, context)
            .map(|provenance| scale_drive_provenance_motive(provenance, motive_scale)),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankedGoalComparisonDimension {
    PriorityClass,
    SubstitutePreferenceOrder,
    Feasibility,
    MotiveScore,
    GoalSpecificity,
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
    left: &AgendaEntry,
    right: &AgendaEntry,
) -> (Ordering, Option<RankedGoalComparisonDimension>) {
    let ordering = right.priority_class.cmp(&left.priority_class);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::PriorityClass));
    }

    let ordering = compare_substitute_preference_order(left, right);
    if ordering != Ordering::Equal {
        return (
            ordering,
            Some(RankedGoalComparisonDimension::SubstitutePreferenceOrder),
        );
    }

    let ordering = right.motive_score.cmp(&left.motive_score);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::MotiveScore));
    }

    let ordering = left.feasibility.cmp(&right.feasibility);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::Feasibility));
    }

    let ordering = compare_goal_specificity(&left.offer.key.kind, &right.offer.key.kind);
    if ordering != Ordering::Equal {
        return (
            ordering,
            Some(RankedGoalComparisonDimension::GoalSpecificity),
        );
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

    let ordering = compare_share_belief_topics(&left.offer.key.kind, &right.offer.key.kind);
    if ordering != Ordering::Equal {
        return (
            ordering,
            Some(RankedGoalComparisonDimension::ShareBeliefTopicOrder),
        );
    }

    let ordering = goal_kind_discriminant(left.offer.key.kind)
        .cmp(&goal_kind_discriminant(right.offer.key.kind));
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::GoalKindOrder));
    }

    let ordering = left.offer.key.commodity.cmp(&right.offer.key.commodity);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::CommodityKey));
    }

    let ordering = left.offer.key.entity.cmp(&right.offer.key.entity);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::EntityKey));
    }

    let ordering = left.offer.key.place.cmp(&right.offer.key.place);
    if ordering != Ordering::Equal {
        return (ordering, Some(RankedGoalComparisonDimension::PlaceKey));
    }

    (Ordering::Equal, None)
}

fn compare_substitute_preference_order(left: &AgendaEntry, right: &AgendaEntry) -> Ordering {
    let (
        GoalKind::AcquireCommodity {
            commodity: left_commodity,
            purpose: CommodityPurpose::SelfConsume,
            quantity: _,
        },
        GoalKind::AcquireCommodity {
            commodity: right_commodity,
            purpose: CommodityPurpose::SelfConsume,
            quantity: _,
        },
    ) = (&left.offer.key.kind, &right.offer.key.kind)
    else {
        return Ordering::Equal;
    };

    if left_commodity.spec().trade_category != right_commodity.spec().trade_category {
        return Ordering::Equal;
    }

    let left_rank = left
        .provenance
        .as_ref()
        .and_then(drive_provenance)
        .and_then(|provenance| provenance.commodity_preference_rank);
    let right_rank = right
        .provenance
        .as_ref()
        .and_then(drive_provenance)
        .and_then(|provenance| provenance.commodity_preference_rank);

    match (left_rank, right_rank) {
        (Some(left_rank), Some(right_rank)) => left_rank.cmp(&right_rank),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn drive_provenance(provenance: &RankedGoalProvenance) -> Option<&RankedDriveGoalProvenance> {
    match provenance {
        RankedGoalProvenance::Drive(provenance) => Some(provenance),
        RankedGoalProvenance::Danger(_) => None,
    }
}

pub(crate) fn explain_ranked_goal_order(
    left: &AgendaEntry,
    right: &AgendaEntry,
) -> Option<RankedGoalComparison> {
    let (ordering, decisive_dimension) = ranked_goal_ordering(left, right);
    let decisive_dimension = decisive_dimension?;
    let (winner, loser) = match ordering {
        Ordering::Less => (
            OpportunityKey {
                goal_key: left.offer.key,
                anchor: left.offer.anchor,
            },
            OpportunityKey {
                goal_key: right.offer.key,
                anchor: right.offer.anchor,
            },
        ),
        Ordering::Greater => (
            OpportunityKey {
                goal_key: right.offer.key,
                anchor: right.offer.anchor,
            },
            OpportunityKey {
                goal_key: left.offer.key,
                anchor: left.offer.anchor,
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

fn compare_goal_specificity(left: &GoalKind, right: &GoalKind) -> Ordering {
    match (left, right) {
        (
            GoalKind::LootCorpse { .. },
            GoalKind::AcquireCommodity {
                purpose: CommodityPurpose::SelfConsume,
                ..
            },
        ) => Ordering::Less,
        (
            GoalKind::AcquireCommodity {
                purpose: CommodityPurpose::SelfConsume,
                ..
            },
            GoalKind::LootCorpse { .. },
        ) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

fn compare_ranked_goals(left: &AgendaEntry, right: &AgendaEntry) -> Ordering {
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

fn opportunity_strength(goal: &AgendaEntry) -> u32 {
    match (&goal.offer.key.kind, goal.provenance.as_ref()) {
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
            | GoalKind::ProduceCommodity { .. }
            | GoalKind::LootCorpse { .. },
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
        worldwake_core::InstitutionalClaim::MissingPersonStatus { .. } => 7,
    }
}

fn goal_kind_discriminant(kind: GoalKind) -> u8 {
    match kind {
        GoalKind::ConsumeOwnedCommodity { .. } => 0,
        GoalKind::AcquireCommodity { .. } => 1,
        GoalKind::Sleep => 2,
        GoalKind::Relieve => 3,
        GoalKind::Wash => 4,
        GoalKind::FreeCarryCapacity => 5,
        GoalKind::EngageHostile { .. } => 6,
        GoalKind::RaidTarget { .. } => 7,
        GoalKind::ReduceDanger => 8,
        GoalKind::RegroupWithFaction { .. } => 9,
        GoalKind::EstablishBanditCamp { .. } => 10,
        GoalKind::TreatWounds { .. } => 11,
        GoalKind::ProduceCommodity { .. } => 12,
        GoalKind::SellCommodity { .. } => 13,
        GoalKind::RestockCommodity { .. } => 14,
        GoalKind::MoveCargo { .. } => 15,
        GoalKind::LootCorpse { .. } => 16,
        GoalKind::BuryCorpse { .. } => 17,
        GoalKind::FulfillBounty { .. } => 18,
        GoalKind::ShareBelief { .. } => 19,
        GoalKind::ClaimOffice { .. } => 20,
        GoalKind::SupportCandidateForOffice { .. } => 21,
        GoalKind::InvestigateViolation { .. } => 22,
        GoalKind::Patrol { .. } => 23,
        GoalKind::StealItem { .. } => 24,
        GoalKind::Accuse { .. } => 25,
        GoalKind::PunishAccused { .. } => 26,
        GoalKind::PostBounty { .. } => 27,
        GoalKind::PostNotice { .. } => 28,
        GoalKind::SearchForMissing { .. } => 29,
        GoalKind::ReportMissing { .. } => 30,
        GoalKind::EscortToSafety { .. } => 31,
        GoalKind::ReportFound { .. } => 32,
        GoalKind::ExploreLocation { .. } => 33,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RankingContext, apply_competition_discount, apply_obligation_satiation,
        apply_source_reliability_discount, build_decision_context,
    };
    use crate::{
        AgendaEntry, ExpectationFailureCause, ExpectationFailurePhase, GoalKey, GoalKind,
        GoalOffer, GoalPriorityClass, OpportunityExpectationFailureIncident,
        OpportunityExpectationKind, RankedDriveGoalProvenance, RankedDriveKind,
        RankedDriveMotiveInput, RankedGoalProvenance, RankedPriorityAdjustment,
        decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, ActionDomain, ArtifactKind, ArtifactPostingContext, ArtifactState,
        BeliefConfidencePolicy, BelievedActivity, BelievedArtifactState, BelievedBountyTerms,
        BelievedEntityState, BelievedInstitutionalClaim, BodyCostPerTick, BodyPart, BountyTarget,
        BountyTerms, CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        CommodityValuationProfile, DemandObservation, DemandObservationReason, DeprivationExposure,
        DeprivationKind, DiversificationProfile, DriveEscalationParams, DriveEscalationProfile,
        DriveThresholds, EffectiveRight, EntityId, EntityKind, EpistemicDispositionProfile,
        ExpectationBasis, ExpectationId, ExpectationRecord, ExpectationState, ExpectationStore,
        GoalRejectionReason, HomeostaticNeedId, HomeostaticNeeds, InTransitOnEdge,
        InstitutionalBeliefRead, InstitutionalClaim, InstitutionalKnowledgeSource,
        JusticeDispositionProfile, LastSeenMemory, LoadUnits, MerchandiseProfile,
        MetabolismProfile, MultiplierPermille, NoticeTopic, ObligationExecutionTracker,
        ObligationSatiationProfile, OfficeData, OpportunityAnchor, PatrolProfile, PatrolRoute,
        PerceptionSource, Permille, PreferenceProfile, ProofRequirement, PunishmentKind, Quantity,
        RecipeId, RecordedViolation, ReliabilityRecord, ResourceSource, RewardSource, RightKind,
        RouteExperience, SourceKey, SourceReliability, SubstitutePreferences, TellTopic,
        TheftDispositionProfile, TheftFacts, Tick, TickRange, TradeCategory,
        TradeDispositionProfile, UniqueItemKind, UtilityProfile, ViolationId, ViolationKind,
        WorkstationTag, Wound, WoundCause, WoundId, belief_confidence,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, CombatBeliefView, ControlBeliefView, DurationExpr,
        EconomicBeliefView, EntityBeliefView, ProfileBeliefView, RecipeDefinition,
        RuntimeBeliefView, SocialBeliefView, SpatialBeliefView, TemporalBeliefView,
        belief_view::{BeliefStatus, BeliefValue},
    };

    #[derive(Clone, Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        place_entities: BTreeMap<EntityId, Vec<EntityId>>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        exposures: BTreeMap<EntityId, DeprivationExposure>,
        escalation_profiles: BTreeMap<EntityId, DriveEscalationProfile>,
        exploration_profiles: BTreeMap<EntityId, worldwake_core::ExplorationProfile>,
        diversification_profiles: BTreeMap<EntityId, DiversificationProfile>,
        last_proactive_exploration_ticks: BTreeMap<EntityId, Tick>,
        obligation_satiation_profiles: BTreeMap<EntityId, ObligationSatiationProfile>,
        obligation_execution_trackers: BTreeMap<EntityId, ObligationExecutionTracker>,
        confidence_policies: BTreeMap<EntityId, BeliefConfidencePolicy>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        courage: BTreeMap<EntityId, Permille>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        hostile_targets: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        commodity_valuation_profiles: BTreeMap<EntityId, CommodityValuationProfile>,
        substitute_preferences: BTreeMap<EntityId, SubstitutePreferences>,
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
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
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
        expectation_stores: BTreeMap<EntityId, ExpectationStore>,
        last_seen_memories: BTreeMap<EntityId, LastSeenMemory>,
        believed_target_locations: BTreeMap<(EntityId, EntityId), BeliefValue<Option<EntityId>>>,
    }

    impl ControlBeliefView for TestBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
            self.believed_rights
                .get(&(actor, entity))
                .cloned()
                .unwrap_or_default()
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EntityBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }
        fn bandit_flee_wound_threshold(&self, faction: EntityId) -> Option<Permille> {
            self.bandit_flee_thresholds.get(&faction).copied()
        }
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn believed_target_location(
            &self,
            agent: EntityId,
            target: EntityId,
        ) -> BeliefValue<Option<EntityId>> {
            self.believed_target_locations
                .get(&(agent, target))
                .copied()
                .unwrap_or_else(|| worldwake_sim::belief_view::stale_default_value(None))
        }
    }

    impl ProfileBeliefView for TestBeliefView {
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure> {
            self.exposures.get(&agent).copied()
        }
        fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile> {
            self.escalation_profiles.get(&agent).cloned()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn exploration_profile(
            &self,
            agent: EntityId,
        ) -> Option<worldwake_core::ExplorationProfile> {
            self.exploration_profiles.get(&agent).copied()
        }
        fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile> {
            self.diversification_profiles.get(&agent).copied()
        }
        fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick> {
            self.last_proactive_exploration_ticks.get(&agent).copied()
        }
        fn obligation_satiation_profile(&self, agent: EntityId) -> ObligationSatiationProfile {
            self.obligation_satiation_profiles
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
        fn obligation_execution_tracker(&self, agent: EntityId) -> ObligationExecutionTracker {
            self.obligation_execution_trackers
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
        fn preference_profile(&self, agent: EntityId) -> Option<PreferenceProfile> {
            self.preference_profiles.get(&agent).copied()
        }
    }

    impl SpatialBeliefView for TestBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.place_entities.get(&place).cloned().unwrap_or_default()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn route_experience(&self, agent: EntityId) -> Option<RouteExperience> {
            self.route_experiences.get(&agent).cloned()
        }

        fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
            self.patrol_routes.get(&agent).cloned()
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
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
    }

    impl TemporalBeliefView for TestBeliefView {
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
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

    impl RuntimeBeliefView for TestBeliefView {}

    impl worldwake_sim::SocialBeliefView for TestBeliefView {
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
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
        fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy {
            *self
                .confidence_policies
                .get(&agent)
                .expect("tests must seed a confidence policy for the acting agent")
        }
        fn source_reliability(&self, agent: EntityId) -> Option<SourceReliability> {
            self.source_reliabilities.get(&agent).cloned()
        }
        fn expectation_store(&self, agent: EntityId) -> Option<ExpectationStore> {
            self.expectation_stores.get(&agent).cloned()
        }
        fn last_seen_memory(&self, agent: EntityId) -> Option<LastSeenMemory> {
            self.last_seen_memories.get(&agent).cloned()
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
        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
    }

    impl worldwake_sim::PoliticalBeliefView for TestBeliefView {
        fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
            self.institutional_claims
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn bandit_factions_of(&self, entity: EntityId) -> Vec<EntityId> {
            self.factions_by_member
                .get(&entity)
                .cloned()
                .unwrap_or_default()
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
        fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
            self.loyalties.get(&(subject, target)).copied()
        }
        fn justice_disposition_profile(
            &self,
            agent: EntityId,
        ) -> Option<JusticeDispositionProfile> {
            self.justice_profiles.get(&agent).cloned()
        }
        fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
            self.active_violation_records
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl CombatBeliefView for TestBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }
        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courage.get(&agent).copied()
        }
        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }
        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }
        fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostile_targets
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }
        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }
        fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
            self.patrol_profiles.get(&agent).cloned()
        }
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
    }

    impl EconomicBeliefView for TestBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn substitute_preferences(&self, agent: EntityId) -> Option<SubstitutePreferences> {
            self.substitute_preferences.get(&agent).cloned()
        }
        fn commodity_valuation_profile(
            &self,
            agent: EntityId,
        ) -> Option<CommodityValuationProfile> {
            self.commodity_valuation_profiles.get(&agent).copied()
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
        fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.listed_sale_lots
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }
        fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
            self.sale_lot_sellers.get(&lot).copied()
        }
        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }
        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }
    }

    impl worldwake_sim::InventoryBeliefView for TestBeliefView {
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
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
        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }
        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }
        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }
        fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
            self.known_recipes.get(&agent).cloned().unwrap_or_default()
        }
        fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
            self.recipe_definitions.get(&recipe).cloned()
        }
    }

    impl worldwake_sim::FacilityBeliefView for TestBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
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
            believed_kind: None,
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
            ..BelievedEntityState::single_observation_defaults(Tick(observed_tick), source)
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

    fn goal(kind: GoalKind) -> GoalOffer {
        GoalOffer {
            anchor: OpportunityAnchor::None,
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
        }
    }

    fn goal_at_place(kind: GoalKind, place: EntityId) -> GoalOffer {
        GoalOffer {
            anchor: OpportunityAnchor::Place(place),
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
        }
    }

    fn goal_at_place_with_sources(
        kind: GoalKind,
        place: EntityId,
        evidence_entities: BTreeSet<EntityId>,
    ) -> GoalOffer {
        GoalOffer {
            anchor: OpportunityAnchor::Place(place),
            key: GoalKey::from(kind),
            evidence_entities,
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
        }
    }

    fn observed_activity_state(
        place: EntityId,
        domain: ActionDomain,
        target: Option<EntityId>,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
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
            ..BelievedEntityState::single_observation_defaults(
                Tick(9),
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn believed_bounty_state(
        issuer: EntityId,
        claim_place: EntityId,
        target: BountyTarget,
        reward_quantity: u32,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
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
            ..BelievedEntityState::single_observation_defaults(
                Tick(9),
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn overdue_expectation(
        id: u64,
        owner: EntityId,
        subject: EntityId,
        expected_place: EntityId,
        deadline_tick: u64,
        basis: ExpectationBasis,
    ) -> ExpectationRecord {
        ExpectationRecord {
            id: ExpectationId(id),
            owner,
            subject,
            expected_place,
            deadline_tick: Tick(deadline_tick),
            grace_ticks: 0,
            basis,
            state: ExpectationState::Overdue,
            created_tick: Tick(deadline_tick.saturating_sub(1)),
        }
    }

    fn expectation_store(records: impl IntoIterator<Item = ExpectationRecord>) -> ExpectationStore {
        let mut store = ExpectationStore::default();
        for record in records {
            store.records.insert(record.id, record);
        }
        store
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

    fn escalation_profile(
        start_after_ticks: u32,
        growth_per_tick: u16,
        max_multiplier: u16,
    ) -> DriveEscalationProfile {
        DriveEscalationProfile {
            per_need: BTreeMap::new(),
            default_per_need: DriveEscalationParams {
                start_after_ticks,
                growth_per_tick: pm(growth_per_tick),
                max_multiplier: MultiplierPermille::new(max_multiplier).unwrap(),
            },
        }
    }

    fn current_tick() -> Tick {
        Tick(10)
    }

    fn food_substitutes(preferences: Vec<CommodityKind>) -> SubstitutePreferences {
        SubstitutePreferences {
            preferences: BTreeMap::from([(TradeCategory::Food, preferences)]),
        }
    }

    fn obligation_profile(
        threshold: u32,
        window_ticks: u32,
        decay_per_execution: u16,
        satiation_floor: u16,
    ) -> ObligationSatiationProfile {
        ObligationSatiationProfile {
            satiation_threshold: threshold,
            window_ticks,
            decay_per_execution: pm(decay_per_execution),
            satiation_floor: pm(satiation_floor),
        }
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

    fn target_location_belief(place: EntityId, confidence: u16) -> BeliefValue<Option<EntityId>> {
        BeliefValue {
            value: Some(place),
            confidence: pm(confidence),
            acquired_tick: current_tick(),
            claimed_event_tick: Some(current_tick()),
            status: BeliefStatus::Certain,
        }
    }

    #[test]
    fn drive_score_preserves_pre_s116_motive_when_counter_below_start_after() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(800), pm(100), pm(100)),
        );
        view.exposures.insert(
            agent,
            DeprivationExposure {
                fatigue_critical_ticks: 100,
                ..DeprivationExposure::default()
            },
        );
        view.escalation_profiles
            .insert(agent, escalation_profile(100, 10, 3000));
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );

        assert_eq!(
            super::drive_score(
                &context,
                HomeostaticNeedId::Fatigue,
                |needs| needs.fatigue,
                |utility| utility.fatigue_weight,
            ),
            super::score_product(utility.fatigue_weight, pm(800))
        );
    }

    #[test]
    fn drive_score_doubles_when_multiplier_is_2000_permille() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(800), pm(100), pm(100)),
        );
        view.exposures.insert(
            agent,
            DeprivationExposure {
                fatigue_critical_ticks: 200,
                ..DeprivationExposure::default()
            },
        );
        view.escalation_profiles
            .insert(agent, escalation_profile(100, 10, 3000));
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );
        let raw = super::score_product(utility.fatigue_weight, pm(800));

        assert_eq!(
            super::drive_score(
                &context,
                HomeostaticNeedId::Fatigue,
                |needs| needs.fatigue,
                |utility| utility.fatigue_weight,
            ),
            raw * 2
        );
    }

    #[test]
    fn drive_score_saturates_at_max_multiplier() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(800), pm(100), pm(100)),
        );
        view.exposures.insert(
            agent,
            DeprivationExposure {
                fatigue_critical_ticks: 1_000,
                ..DeprivationExposure::default()
            },
        );
        view.escalation_profiles
            .insert(agent, escalation_profile(100, 20, 1800));
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );
        let raw = super::score_product(utility.fatigue_weight, pm(800));

        assert_eq!(
            super::drive_score(
                &context,
                HomeostaticNeedId::Fatigue,
                |needs| needs.fatigue,
                |utility| utility.fatigue_weight,
            ),
            raw * 1800 / 1000
        );
    }

    #[test]
    fn relevant_self_consume_factors_attaches_escalation_multiplier_to_hunger_factor() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(900), pm(100), pm(100), pm(100), pm(100)),
        );
        view.exposures.insert(
            agent,
            DeprivationExposure {
                hunger_critical_ticks: 150,
                ..DeprivationExposure::default()
            },
        );
        view.escalation_profiles
            .insert(agent, escalation_profile(100, 10, 3000));
        let utility = utility();
        let context = RankingContext::new(
            &view,
            agent,
            current_tick(),
            &utility,
            build_decision_context(&view, agent),
        );
        let hunger_factor = super::relevant_self_consume_factors(CommodityKind::Bread, &context)
            .into_iter()
            .find(|factor| factor.drive == RankedDriveKind::Hunger)
            .unwrap();

        assert_eq!(
            hunger_factor.escalation_multiplier,
            MultiplierPermille::new(1500).unwrap()
        );
    }

    fn seed_directly_possessed_waste_lot(
        view: &mut TestBeliefView,
        agent: EntityId,
        place: EntityId,
        waste_lot: EntityId,
        quantity: Quantity,
    ) {
        view.entity_kinds.insert(waste_lot, EntityKind::ItemLot);
        view.effective_places.insert(waste_lot, place);
        view.place_entities
            .entry(place)
            .or_default()
            .push(waste_lot);
        view.direct_possessions
            .entry(agent)
            .or_default()
            .push(waste_lot);
        view.direct_possessors.insert(waste_lot, agent);
        view.entity_loads.insert(
            waste_lot,
            LoadUnits(
                quantity
                    .0
                    .saturating_mul(worldwake_core::load_per_unit(CommodityKind::Waste).0),
            ),
        );

        let mut waste_belief = believed_state(9, PerceptionSource::DirectObservation);
        waste_belief.last_known_place = Some(place);
        waste_belief.believed_kind = Some(EntityKind::ItemLot);
        waste_belief
            .last_known_inventory
            .insert(CommodityKind::Waste, quantity);
        view.beliefs
            .entry(agent)
            .or_default()
            .push((waste_lot, waste_belief));
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
                ranked.offer.key.kind
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
                ranked.offer.key.kind
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
                ranked.offer.key.kind
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
    fn survival_relevant_theft_uses_target_commodity_drive_priority_and_motive() {
        let agent = entity(1);
        let item = entity(2);
        let place = entity(99);
        let mut view = base_view(agent);
        view.theft_profiles.insert(
            agent,
            TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(700),
                witness_risk_penalty: pm(150),
            },
        );
        view.entity_kinds.insert(item, EntityKind::ItemLot);
        view.item_lot_commodities.insert(item, CommodityKind::Apple);
        view.commodity_quantities
            .insert((item, CommodityKind::Apple), Quantity(2));
        view.place_entities.insert(place, vec![agent, item]);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(900), pm(100), pm(100), pm(100), pm(100)),
        );

        let ranked = rank(
            &[goal(GoalKind::StealItem { target_item: item })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 567_000);
        match ranked[0]
            .provenance
            .as_ref()
            .expect("steal should carry drive provenance when the target relieves hunger")
        {
            RankedGoalProvenance::Drive(provenance) => {
                assert_eq!(provenance.base_priority_class, GoalPriorityClass::Critical);
                assert_eq!(provenance.final_priority_class, GoalPriorityClass::Critical);
                assert!(!provenance.motive_inputs.is_empty());
                assert!(
                    provenance
                        .motive_inputs
                        .iter()
                        .any(|input| input.drive == RankedDriveKind::Hunger
                            && input.score == 567_000)
                );
            }
            RankedGoalProvenance::Danger(_) => {
                panic!("steal should not use danger provenance")
            }
        }
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
    fn post_bounty_goal_applies_obligation_satiation_decay() {
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
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(2, 48, 200, 50));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(7), Tick(8), Tick(9)],
            },
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

        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].motive_score, 3_360);
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
    fn post_notice_goal_applies_obligation_satiation_decay() {
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
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(1, 48, 300, 100));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(7), Tick(8), Tick(9)],
            },
        );

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

        let raw_score = super::score_product(
            utility.notice_posting_weight,
            super::derive_danger_pressure(&view, agent),
        );
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].motive_score, raw_score * 400 / 1000);
    }

    #[test]
    fn apply_obligation_satiation_returns_raw_score_without_recent_executions() {
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

        assert_eq!(apply_obligation_satiation(&context, 808_200), 808_200);
    }

    #[test]
    fn apply_obligation_satiation_returns_raw_score_at_threshold() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(2, 48, 200, 50));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(8), Tick(9)],
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

        assert_eq!(apply_obligation_satiation(&context, 808_200), 808_200);
    }

    #[test]
    fn apply_obligation_satiation_decays_above_threshold() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(2, 48, 200, 50));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(7), Tick(8), Tick(9)],
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

        assert_eq!(apply_obligation_satiation(&context, 808_200), 646_560);
    }

    #[test]
    fn apply_obligation_satiation_respects_floor() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(1, 48, 300, 100));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(5), Tick(6), Tick(7), Tick(8), Tick(9)],
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

        assert_eq!(apply_obligation_satiation(&context, 1_000), 100);
    }

    #[test]
    fn apply_obligation_satiation_default_profile_matches_spec_arithmetic() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.obligation_satiation_profiles
            .insert(agent, ObligationSatiationProfile::default());
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![
                    Tick(4),
                    Tick(5),
                    Tick(6),
                    Tick(7),
                    Tick(8),
                    Tick(9),
                    Tick(10),
                ],
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

        assert_eq!(apply_obligation_satiation(&context, 808_200), 40_410);
    }

    #[test]
    fn ranking_context_prunes_stale_obligation_execution_ticks() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.obligation_satiation_profiles
            .insert(agent, obligation_profile(2, 3, 200, 50));
        view.obligation_execution_trackers.insert(
            agent,
            ObligationExecutionTracker {
                completion_ticks: vec![Tick(6), Tick(7), Tick(8), Tick(10)],
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
            context.obligation_tracker.completion_ticks,
            vec![Tick(7), Tick(8), Tick(10)]
        );
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
        candidates: &[GoalOffer],
        view: &TestBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &UtilityProfile,
    ) -> super::RankingOutcome {
        rank_with_memories(
            candidates,
            view,
            agent,
            current_tick,
            utility,
            &worldwake_core::RepairMemory::default(),
            &worldwake_core::LearnedOpportunityMemory::default(),
        )
    }

    fn rank_with_memories(
        candidates: &[GoalOffer],
        view: &TestBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &UtilityProfile,
        repair_memory: &worldwake_core::RepairMemory,
        learned_opportunity_memory: &worldwake_core::LearnedOpportunityMemory,
    ) -> super::RankingOutcome {
        let dc = build_decision_context(view, agent);
        super::rank_candidates_with_memories(
            candidates,
            view,
            agent,
            current_tick,
            utility,
            dc,
            repair_memory,
            learned_opportunity_memory,
        )
    }

    #[test]
    fn suppressed_candidates_record_stress_policy_reason() {
        let agent = entity(1);
        let corpse = entity(2);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(900), pm(100), pm(100), pm(100), pm(100)),
        );

        let outcome = rank(
            &[goal(GoalKind::LootCorpse { corpse })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.ranked.is_empty());
        assert_eq!(
            outcome.suppressed,
            vec![
                crate::candidate_generation::CandidateSuppressionDiagnostic {
                    opportunity: worldwake_core::OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::LootCorpse { corpse }),
                        anchor: OpportunityAnchor::None,
                    },
                    reason: GoalRejectionReason::SuppressedByStressPolicy,
                }
            ]
        );
    }

    #[test]
    fn survival_relevant_theft_is_not_suppressed_by_self_care_stress() {
        let agent = entity(1);
        let place = entity(10);
        let item = entity(20);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(900), pm(100), pm(100), pm(100), pm(100)),
        );
        view.theft_profiles.insert(
            agent,
            TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(3).unwrap(),
                theft_motive_weight: pm(700),
                witness_risk_penalty: pm(0),
            },
        );
        view.entity_kinds.insert(item, EntityKind::ItemLot);
        view.item_lot_commodities.insert(item, CommodityKind::Apple);
        view.commodity_quantities
            .insert((item, CommodityKind::Apple), Quantity(2));
        view.place_entities.insert(place, vec![agent, item]);

        let outcome = rank(
            &[goal(GoalKind::StealItem { target_item: item })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.suppressed.is_empty());
        let ranked = outcome.into_ranked();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert!(ranked[0].motive_score > 0);
    }

    #[test]
    fn repair_memory_boosts_matching_alternative_only_while_live() {
        let agent = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let goal_kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let candidates = [
            goal_at_place(goal_kind, place_a),
            goal_at_place(goal_kind, place_b),
        ];
        let view = base_view(agent);
        let utility = utility();
        let baseline = rank(&candidates, &view, agent, current_tick(), &utility).into_ranked();
        let mut repair_memory = worldwake_core::RepairMemory::default();
        repair_memory.record(worldwake_core::RepairEntry {
            repair_key: worldwake_core::RepairKey {
                goal_key: worldwake_core::GoalKey::from(goal_kind),
                alternate_target: place_b,
            },
            observed_tick: Tick(2),
            expires_tick: Tick(20),
            success_count: 1,
        });

        let boosted = rank_with_memories(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility,
            &repair_memory,
            &worldwake_core::LearnedOpportunityMemory::default(),
        )
        .into_ranked();
        let expired = rank_with_memories(
            &candidates,
            &view,
            agent,
            Tick(21),
            &utility,
            &repair_memory,
            &worldwake_core::LearnedOpportunityMemory::default(),
        )
        .into_ranked();

        assert_eq!(baseline[0].offer.anchor, OpportunityAnchor::Place(place_a));
        assert_eq!(boosted[0].offer.anchor, OpportunityAnchor::Place(place_b));
        assert_eq!(expired[0].offer.anchor, OpportunityAnchor::Place(place_a));
    }

    #[test]
    fn learned_opportunity_memory_boosts_matching_opportunity_only_while_live() {
        let agent = entity(1);
        let place_a = entity(20);
        let place_b = entity(21);
        let goal_kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let candidates = [
            goal_at_place(goal_kind, place_a),
            goal_at_place(goal_kind, place_b),
        ];
        let view = base_view(agent);
        let utility = utility();
        let mut learned = worldwake_core::LearnedOpportunityMemory::default();
        learned.record(worldwake_core::OpportunityEntry {
            opportunity: worldwake_core::OpportunityKey {
                goal_key: worldwake_core::GoalKey::from(goal_kind),
                anchor: OpportunityAnchor::Place(place_b),
            },
            observed_tick: Tick(3),
            expires_tick: Tick(18),
            observed_at: place_b,
        });

        let boosted = rank_with_memories(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility,
            &worldwake_core::RepairMemory::default(),
            &learned,
        )
        .into_ranked();
        let expired = rank_with_memories(
            &candidates,
            &view,
            agent,
            Tick(19),
            &utility,
            &worldwake_core::RepairMemory::default(),
            &learned,
        )
        .into_ranked();

        assert_eq!(boosted[0].offer.anchor, OpportunityAnchor::Place(place_b));
        assert_eq!(expired[0].offer.anchor, OpportunityAnchor::Place(place_a));
    }

    #[test]
    fn search_for_missing_goal_has_non_zero_motive_for_overdue_expectation() {
        let agent = entity(1);
        let subject = entity(2);
        let place = entity(10);
        let mut view = base_view(agent);
        view.expectation_stores.insert(
            agent,
            expectation_store([overdue_expectation(
                1,
                agent,
                subject,
                place,
                4,
                ExpectationBasis::RoutineReturn,
            )]),
        );

        let outcome = rank(
            &[goal(GoalKind::SearchForMissing {
                subject,
                last_seen: Some(place),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.zero_motive.is_empty());
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].motive_score, 200 * 6);
    }

    #[test]
    fn report_missing_goal_has_non_zero_motive_for_overdue_expectation() {
        let agent = entity(1);
        let subject = entity(2);
        let place = entity(10);
        let mut view = base_view(agent);
        view.expectation_stores.insert(
            agent,
            expectation_store([overdue_expectation(
                1,
                agent,
                subject,
                place,
                7,
                ExpectationBasis::RoutineReturn,
            )]),
        );

        let outcome = rank(
            &[goal(GoalKind::ReportMissing {
                subject,
                to_office: None,
                expectation_id: None,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.zero_motive.is_empty());
        assert_eq!(outcome.ranked.len(), 1);
        assert_eq!(outcome.ranked[0].motive_score, 150 * 3);
    }

    #[test]
    fn duty_assignment_missing_search_outranks_social_promise() {
        let agent = entity(1);
        let subject = entity(2);
        let place = entity(10);
        let mut view = base_view(agent);
        view.expectation_stores.insert(
            agent,
            expectation_store([
                overdue_expectation(1, agent, subject, place, 8, ExpectationBasis::SocialPromise),
                overdue_expectation(
                    2,
                    agent,
                    subject,
                    place,
                    8,
                    ExpectationBasis::DutyAssignment { office: entity(40) },
                ),
            ]),
        );

        let ranked = rank(
            &[goal(GoalKind::SearchForMissing {
                subject,
                last_seen: Some(place),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].motive_score, 200 * 6);
    }

    #[test]
    fn missing_response_goals_are_zero_motive_without_matching_overdue_expectation() {
        let agent = entity(1);
        let subject = entity(2);
        let view = base_view(agent);

        let outcome = rank(
            &[
                goal(GoalKind::SearchForMissing {
                    subject,
                    last_seen: None,
                }),
                goal(GoalKind::ReportMissing {
                    subject,
                    to_office: None,
                    expectation_id: None,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert_eq!(
            outcome.zero_motive,
            vec![
                GoalKey::from(GoalKind::SearchForMissing {
                    subject,
                    last_seen: None,
                }),
                GoalKey::from(GoalKind::ReportMissing {
                    subject,
                    to_office: None,
                    expectation_id: None,
                }),
            ]
        );
        assert!(outcome.ranked.is_empty());
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
                    quantity: AcquisitionQuantity::single(),
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
                        quantity: AcquisitionQuantity::single(),
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
                        quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
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
                        quantity: AcquisitionQuantity::single(),
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
    fn motive_score_falls_back_to_success_ratio_for_acquire_commodity() {
        // Two believed sources for the same commodity with different
        // reliability records (3-success/1-failure vs 1-success/3-failure).
        // The ranking pipeline's `apply_source_reliability_discount` uses
        // `failure_ratio_permille` (= 1 - success_ratio) as the discount,
        // so the source with the higher success ratio retains a higher
        // post-discount motive — i.e. ranks higher. This is the spec D8
        // "fallback to success_ratio without S131" contract.
        let agent = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let source_a = entity(20);
        let source_b = entity(21);
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
                sources: BTreeMap::from([
                    (
                        SourceKey {
                            entity: source_a,
                            commodity: CommodityKind::Bread,
                        },
                        // 75% success → 250 permille failure ratio.
                        source_reliability_record(3, 1),
                    ),
                    (
                        SourceKey {
                            entity: source_b,
                            commodity: CommodityKind::Bread,
                        },
                        // 25% success → 750 permille failure ratio.
                        source_reliability_record(1, 3),
                    ),
                ]),
            },
        );
        // Hunger pressure to give the AcquireCommodity goal nonzero motive.
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );

        let goal_kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let ranked = rank(
            &[
                goal_at_place_with_sources(goal_kind, place_a, BTreeSet::from([source_a])),
                goal_at_place_with_sources(goal_kind, place_b, BTreeSet::from([source_b])),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 2);
        let entry_a = ranked
            .iter()
            .find(|entry| entry.offer.evidence_entities.contains(&source_a))
            .expect("entry for source_a exists");
        let entry_b = ranked
            .iter()
            .find(|entry| entry.offer.evidence_entities.contains(&source_b))
            .expect("entry for source_b exists");
        // Discount produced for each source:
        let discount_a = entry_a.source_reliability_discount.as_ref().unwrap();
        let discount_b = entry_b.source_reliability_discount.as_ref().unwrap();
        assert_eq!(discount_a.failure_ratio_permille, 250);
        assert_eq!(discount_b.failure_ratio_permille, 750);
        // Higher-success-ratio source retains a higher post-discount motive,
        // and thus ranks higher (lower index) in the output ordering.
        assert!(
            entry_a.motive_score > entry_b.motive_score,
            "75%-success source should rank above 25%-success source: a={}, b={}",
            entry_a.motive_score,
            entry_b.motive_score,
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
    fn pending_source_reliability_failure_reorders_candidates_before_persistence() {
        let agent = entity(1);
        let familiar_place = entity(2);
        let novel_place = entity(3);
        let familiar_source = entity(50);
        let novel_source = entity(51);
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
                        entity: familiar_source,
                        commodity: CommodityKind::Bread,
                    },
                    source_reliability_record(1, 0),
                )]),
            },
        );

        let mut ranked = rank(
            &[
                goal_at_place_with_sources(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    familiar_place,
                    BTreeSet::from([familiar_source]),
                ),
                goal_at_place_with_sources(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    novel_place,
                    BTreeSet::from([novel_source]),
                ),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(
            ranked[0].offer.anchor,
            OpportunityAnchor::Place(familiar_place)
        );
        assert_eq!(ranked[0].source_reliability_discount, None);
        let familiar_opportunity = worldwake_core::OpportunityKey {
            goal_key: ranked[0].offer.key,
            anchor: ranked[0].offer.anchor,
        };

        super::apply_pending_source_reliability_failures(
            &mut ranked,
            &super::PendingSourceReliabilityInputs {
                view: &view,
                agent,
                current_tick: current_tick(),
                utility: &utility(),
                decision_context: build_decision_context(&view, agent),
                repair_memory: super::empty_repair_memory(),
                learned_opportunity_memory: super::empty_learned_opportunity_memory(),
            },
            &[OpportunityExpectationFailureIncident {
                opportunity: familiar_opportunity,
                source: SourceKey {
                    entity: familiar_source,
                    commodity: CommodityKind::Bread,
                },
                expectation_kind: OpportunityExpectationKind::AcquireCommodityFromConcreteSource,
                detected_at_tick: current_tick(),
                phase: ExpectationFailurePhase::Observation,
                cause: ExpectationFailureCause::SourceDepletedLocally,
            }],
        );

        assert_eq!(
            ranked[0].offer.anchor,
            OpportunityAnchor::Place(novel_place)
        );
        assert_eq!(
            ranked[1].source_reliability_discount,
            Some(SourceReliabilityDiscount {
                source_entity: familiar_source,
                commodity: CommodityKind::Bread,
                failure_ratio_permille: 500,
                pre_discount_motive: 90_000,
                post_discount_motive: 45_000,
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
                quantity: AcquisitionQuantity::single(),
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
    fn loot_corpse_with_recovery_relevant_food_uses_self_care_priority() {
        let agent = entity(1);
        let corpse = entity(3);
        let mut view = base_view(agent);
        view.commodity_quantities
            .insert((corpse, CommodityKind::Apple), Quantity(4));
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(600), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank(
            &[goal(GoalKind::LootCorpse { corpse })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert!(
            ranked[0].priority_class > GoalPriorityClass::Low,
            "recovery-relevant corpse loot should no longer stay in the fixed low-priority bucket"
        );
        assert!(
            ranked[0].motive_score > 1,
            "recovery-relevant corpse loot should carry a real motive score"
        );
    }

    #[test]
    fn loot_corpse_drive_provenance_participates_in_opportunity_strength_tiebreak() {
        let corpse = entity(3);
        let acquire = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }),
                anchor: OpportunityAnchor::None,
            },
            offer: goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            priority_class: GoalPriorityClass::Medium,
            motive_score: 250_000,
            provenance: Some(RankedGoalProvenance::Drive(RankedDriveGoalProvenance {
                base_priority_class: GoalPriorityClass::Medium,
                final_priority_class: GoalPriorityClass::Medium,
                adjustment: None,
                commodity_preference_rank: None,
                motive_inputs: vec![RankedDriveMotiveInput {
                    drive: RankedDriveKind::Hunger,
                    pressure: pm(500),
                    weight: pm(500),
                    score: 250_000,
                    escalation_multiplier: MultiplierPermille::IDENTITY,
                    relief_per_unit: pm(600),
                    recovery_relevant: true,
                }],
            })),
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };
        let loot = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::LootCorpse { corpse }),
                anchor: OpportunityAnchor::None,
            },
            offer: goal(GoalKind::LootCorpse { corpse }),
            priority_class: GoalPriorityClass::Medium,
            motive_score: 250_000,
            provenance: Some(RankedGoalProvenance::Drive(RankedDriveGoalProvenance {
                base_priority_class: GoalPriorityClass::Medium,
                final_priority_class: GoalPriorityClass::Medium,
                adjustment: None,
                commodity_preference_rank: None,
                motive_inputs: vec![RankedDriveMotiveInput {
                    drive: RankedDriveKind::Hunger,
                    pressure: pm(500),
                    weight: pm(500),
                    score: 250_000,
                    escalation_multiplier: MultiplierPermille::IDENTITY,
                    relief_per_unit: pm(900),
                    recovery_relevant: true,
                }],
            })),
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };

        assert_eq!(
            super::compare_ranked_goals(&loot, &acquire),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn loot_corpse_outranks_generic_self_consume_acquire_when_other_factors_tie() {
        let corpse = entity(3);
        let acquire = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }),
                anchor: OpportunityAnchor::None,
            },
            offer: goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            priority_class: GoalPriorityClass::Medium,
            motive_score: 250_000,
            provenance: Some(RankedGoalProvenance::Drive(RankedDriveGoalProvenance {
                base_priority_class: GoalPriorityClass::Medium,
                final_priority_class: GoalPriorityClass::Medium,
                adjustment: None,
                commodity_preference_rank: None,
                motive_inputs: vec![RankedDriveMotiveInput {
                    drive: RankedDriveKind::Hunger,
                    pressure: pm(500),
                    weight: pm(500),
                    score: 250_000,
                    escalation_multiplier: MultiplierPermille::IDENTITY,
                    relief_per_unit: pm(600),
                    recovery_relevant: true,
                }],
            })),
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Likely,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };
        let loot = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::LootCorpse { corpse }),
                anchor: OpportunityAnchor::None,
            },
            offer: goal(GoalKind::LootCorpse { corpse }),
            priority_class: GoalPriorityClass::Medium,
            motive_score: 250_000,
            provenance: Some(RankedGoalProvenance::Drive(RankedDriveGoalProvenance {
                base_priority_class: GoalPriorityClass::Medium,
                final_priority_class: GoalPriorityClass::Medium,
                adjustment: None,
                commodity_preference_rank: None,
                motive_inputs: vec![RankedDriveMotiveInput {
                    drive: RankedDriveKind::Hunger,
                    pressure: pm(500),
                    weight: pm(500),
                    score: 250_000,
                    escalation_multiplier: MultiplierPermille::IDENTITY,
                    relief_per_unit: pm(400),
                    recovery_relevant: true,
                }],
            })),
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Likely,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };

        assert_eq!(
            super::compare_ranked_goals(&loot, &acquire),
            std::cmp::Ordering::Less
        );
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
            ranked[0].offer.key.kind,
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
            enterprise_first[0].offer.key.kind,
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
            self_care_first[0].offer.key.kind,
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
            ranked.first().map(|goal| goal.offer.key.kind),
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
            ranked[0].offer.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].offer.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(
            ranked[2].offer.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_a
        ));
        assert!(matches!(
            ranked[3].offer.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_b
        ));
    }

    #[test]
    fn substitute_preference_order_outranks_same_category_self_consume_rival() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );
        view.substitute_preferences.insert(
            agent,
            food_substitutes(vec![CommodityKind::Grain, CommodityKind::Apple]),
        );

        let ranked = rank(
            &[
                goal_at_place(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Apple,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    market,
                ),
                goal_at_place(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Grain,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    market,
                ),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert!(matches!(
            ranked[0].offer.key.kind,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Grain,
                purpose: CommodityPurpose::SelfConsume,
                quantity: _,
            }
        ));
        assert_eq!(
            super::explain_ranked_goal_order(&ranked[0], &ranked[1])
                .expect("ranked goals should explain their ordering")
                .decisive_dimension,
            super::RankedGoalComparisonDimension::SubstitutePreferenceOrder
        );
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

        assert_eq!(ranked[0].offer.key.kind, GoalKind::TreatWounds { patient });
        assert_eq!(ranked[0].motive_score, 900 * 500);
        assert_eq!(
            ranked[1].offer.key.kind,
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
            ranked[0].offer.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].motive_score, 900 * 500);
        assert_eq!(ranked[1].offer.key.kind, GoalKind::TreatWounds { patient });
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
            ranked[0].offer.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].offer.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(ranked[2].offer.key.kind, GoalKind::Sleep));
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
            bread.offer.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }
        );
        assert_eq!(bread.priority_class, GoalPriorityClass::Critical);
        assert_eq!(bread.motive_score, 380_000);

        assert_eq!(wash.offer.key.kind, GoalKind::Wash);
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
    fn free_carry_capacity_uses_low_priority_class() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let place = view.effective_places[&agent];
        let waste_lot = entity(20);
        view.carry_capacities.insert(agent, LoadUnits(10));
        view.commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(8));
        seed_directly_possessed_waste_lot(&mut view, agent, place, waste_lot, Quantity(8));

        let ranked = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
    }

    #[test]
    fn free_carry_capacity_motive_scales_with_carried_load_strain() {
        let agent = entity(1);
        let fifty_lot = entity(20);
        let mut fifty_view = base_view(agent);
        let fifty_place = fifty_view.effective_places[&agent];
        fifty_view.carry_capacities.insert(agent, LoadUnits(10));
        fifty_view
            .commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(8));
        seed_directly_possessed_waste_lot(
            &mut fifty_view,
            agent,
            fifty_place,
            fifty_lot,
            Quantity(8),
        );

        let fifty_ranked = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &fifty_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(fifty_ranked.len(), 1);
        assert_eq!(
            fifty_ranked[0].motive_score,
            super::score_product(utility().enterprise_weight, pm(800))
        );

        let full_lot = entity(21);
        let mut full_view = base_view(agent);
        let full_place = full_view.effective_places[&agent];
        full_view.carry_capacities.insert(agent, LoadUnits(10));
        full_view
            .commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(10));
        seed_directly_possessed_waste_lot(
            &mut full_view,
            agent,
            full_place,
            full_lot,
            Quantity(10),
        );

        let full_ranked = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &full_view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(full_ranked.len(), 1);
        assert_eq!(
            full_ranked[0].motive_score,
            super::score_product(utility().enterprise_weight, pm(1000))
        );
        assert!(full_ranked[0].motive_score > fifty_ranked[0].motive_score);
    }

    #[test]
    fn free_carry_capacity_motive_uses_actor_carried_load_accessor() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let place = view.effective_places[&agent];
        let waste_lot = entity(20);
        view.carry_capacities.insert(agent, LoadUnits(10));
        view.commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(18));
        seed_directly_possessed_waste_lot(&mut view, agent, place, waste_lot, Quantity(8));

        let ranked = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            ranked[0].motive_score,
            super::score_product(utility().enterprise_weight, pm(800))
        );
    }

    #[test]
    fn free_carry_capacity_motive_is_zero_when_capacity_unavailable() {
        let agent = entity(1);
        let mut view = base_view(agent);
        view.commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(8));

        let outcome = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(outcome.ranked.is_empty());
        assert_eq!(
            outcome.zero_motive,
            vec![GoalKey::from(GoalKind::FreeCarryCapacity)]
        );
    }

    #[test]
    fn free_carry_capacity_motive_is_zero_when_disposal_is_not_actionable() {
        let agent = entity(1);
        let mut below_threshold_view = base_view(agent);
        let below_threshold_place = below_threshold_view.effective_places[&agent];
        below_threshold_view
            .carry_capacities
            .insert(agent, LoadUnits(10));
        below_threshold_view
            .commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(7));
        seed_directly_possessed_waste_lot(
            &mut below_threshold_view,
            agent,
            below_threshold_place,
            entity(20),
            Quantity(7),
        );

        let below_threshold_outcome = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &below_threshold_view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(below_threshold_outcome.ranked.is_empty());
        assert_eq!(
            below_threshold_outcome.zero_motive,
            vec![GoalKey::from(GoalKind::FreeCarryCapacity)]
        );

        let mut no_target_view = base_view(agent);
        no_target_view.carry_capacities.insert(agent, LoadUnits(10));
        no_target_view
            .commodity_quantities
            .insert((agent, CommodityKind::Waste), Quantity(9));

        let no_target_outcome = rank(
            &[goal(GoalKind::FreeCarryCapacity)],
            &no_target_view,
            agent,
            current_tick(),
            &utility(),
        );

        assert!(no_target_outcome.ranked.is_empty());
        assert_eq!(
            no_target_outcome.zero_motive,
            vec![GoalKey::from(GoalKind::FreeCarryCapacity)]
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
        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(view.effective_places[&agent], 1000),
        );
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
    fn engage_hostile_remote_target_has_low_danger_motive() {
        let agent = entity(1);
        let target = entity(7);
        let mut view = base_view(agent);
        view.hostile_targets.insert(agent, vec![target]);

        let ranked = rank(
            &[goal(GoalKind::EngageHostile { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(
            ranked[0].motive_score,
            u32::from(utility().danger_weight.value())
                * u32::from(DriveThresholds::default().danger.low().value())
        );
        match ranked[0]
            .provenance
            .as_ref()
            .expect("engage hostile should carry danger provenance")
        {
            RankedGoalProvenance::Danger(assessment) => {
                assert_eq!(assessment.visible_hostiles, Vec::<EntityId>::new());
                assert_eq!(assessment.hostile_targets, vec![target]);
            }
            RankedGoalProvenance::Drive(_) => {
                panic!("engage hostile should not use drive provenance")
            }
        }
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
        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(view.effective_places[&agent], 1000),
        );
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
        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(view.effective_places[&agent], 1000),
        );
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
    fn raid_target_motive_scales_with_target_location_confidence() {
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

        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(view.effective_places[&agent], 500),
        );
        let half = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();
        assert_eq!(half.len(), 1);
        assert_eq!(
            half[0].motive_score,
            4 * u32::from(utility().hunger_weight.value()) * 350
        );

        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(view.effective_places[&agent], 0),
        );
        let zero = rank(
            &[goal(GoalKind::RaidTarget { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
        );
        assert!(zero.into_ranked().is_empty());
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
                    believed_kind: None,
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
                    ..worldwake_core::BelievedEntityState::single_observation_defaults(
                        current_tick(),
                        PerceptionSource::DirectObservation,
                    )
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
            ranked[0].offer.key.kind,
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
            ranked[0].offer.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 850);
        assert_eq!(ranked[1].offer.key.kind, GoalKind::ClaimOffice { office });
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
            ranked[0].offer.key.kind,
            GoalKind::TreatWounds { patient: agent }
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 1050);
        assert_eq!(ranked[1].offer.key.kind, GoalKind::ClaimOffice { office });
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

        assert_eq!(ranked[0].offer.key.kind, GoalKind::ClaimOffice { office });
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 1);
        assert_eq!(
            ranked[1].offer.key.kind,
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
    ) -> crate::AgendaEntry {
        crate::AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey {
                    kind,
                    commodity: None,
                    entity: None,
                    place: None,
                },
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey {
                    kind,
                    commodity: None,
                    entity: None,
                    place: None,
                },
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
            },
            priority_class,
            motive_score: motive,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    #[test]
    fn ordered_ranked_exposes_len_and_first_in_sorted_order() {
        use crate::feasibility::FeasibilityHint;

        let ranked = vec![
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::Critical,
                900,
                FeasibilityHint::Likely,
            ),
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::High,
                500,
                FeasibilityHint::Likely,
            ),
            make_ranked_goal(
                GoalKind::Patrol { place: entity(50) },
                GoalPriorityClass::Low,
                100,
                FeasibilityHint::Likely,
            ),
        ];

        let ordered = super::OrderedRanked::from_sorted_for_test(&ranked);

        assert!(!ordered.is_empty());
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered.first(), Some(&ranked[0]));
        assert_eq!(ordered.iter().cloned().collect::<Vec<_>>(), ranked);
    }

    #[test]
    fn ordered_ranked_find_returns_first_match() {
        use crate::feasibility::FeasibilityHint;

        let ranked = vec![
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::Critical,
                900,
                FeasibilityHint::Likely,
            ),
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::High,
                700,
                FeasibilityHint::Likely,
            ),
            make_ranked_goal(
                GoalKind::Sleep,
                GoalPriorityClass::Low,
                300,
                FeasibilityHint::Likely,
            ),
        ];

        let ordered = super::OrderedRanked::from_sorted_for_test(&ranked);
        let found = ordered.find(|goal| matches!(goal.offer.key.kind, GoalKind::Sleep));

        assert_eq!(found, Some(&ranked[1]));
    }

    #[test]
    fn sort_in_place_matches_ranker_output() {
        let agent = entity(1);
        let orchard = entity(2);
        let well = entity(3);
        let camp = entity(4);
        let view = base_view(agent);
        let candidates = vec![
            goal_at_place(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                orchard,
            ),
            goal_at_place(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                well,
            ),
            goal_at_place(GoalKind::Sleep, camp),
        ];

        let expected = rank(&candidates, &view, agent, current_tick(), &utility()).into_ranked();
        let mut actual = expected.iter().rev().cloned().collect::<Vec<_>>();
        let ordered = super::sort_in_place(&mut actual);

        assert_eq!(ordered.as_slice(), expected.as_slice());
        assert_eq!(actual, expected);
    }

    #[test]
    fn ranking_outcome_ordered_reflects_ranked_field() {
        let agent = entity(1);
        let orchard = entity(2);
        let well = entity(3);
        let view = base_view(agent);
        let candidates = vec![
            goal_at_place(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                orchard,
            ),
            goal_at_place(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                well,
            ),
            goal_at_place(GoalKind::Wash, orchard),
        ];

        let outcome = rank(&candidates, &view, agent, current_tick(), &utility());

        assert_eq!(outcome.ordered().as_slice(), outcome.ranked.as_slice());
    }

    #[test]
    fn compare_ranked_goals_is_the_only_impl_in_crate() {
        use std::fs;
        use std::path::Path;

        fn walk(dir: &Path, offending: &mut Vec<String>, needle: &[u8]) {
            for entry in fs::read_dir(dir).expect("read_dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, offending, needle);
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                if path.file_name().and_then(|s| s.to_str()) == Some("ranking.rs") {
                    continue;
                }
                let bytes = fs::read(&path).expect("read file");
                if bytes.windows(needle.len()).any(|window| window == needle) {
                    offending.push(path.display().to_string());
                }
            }
        }

        let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let needle = b"fn compare_ranked_goals";
        let mut offending = Vec::new();

        walk(&src_root, &mut offending, needle);

        assert!(
            offending.is_empty(),
            "`fn compare_ranked_goals` must only be defined in ranking.rs; found parallel definitions in: {offending:?}"
        );
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
        // Within one priority class, higher motive should outrank a cheaper
        // feasibility hint so urgent remote self-care is not starved by a
        // merely local sibling option.
        assert_eq!(goals[0].motive_score, 900);
        assert_eq!(goals[0].feasibility, FeasibilityHint::Unlikely);
        assert_eq!(goals[1].motive_score, 600);
        assert_eq!(goals[1].feasibility, FeasibilityHint::Likely);
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
    fn critical_remote_food_can_outrank_local_wash_on_motive() {
        use crate::feasibility::FeasibilityHint;
        let mut goals = [
            make_ranked_goal(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::Critical,
                617_400,
                FeasibilityHint::Uncertain,
            ),
            make_ranked_goal(
                GoalKind::Wash,
                GoalPriorityClass::Critical,
                600_000,
                FeasibilityHint::Likely,
            ),
        ];

        goals.sort_by(super::compare_ranked_goals);

        assert!(matches!(
            goals[0].offer.key.kind,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: _,
            }
        ));
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

    #[test]
    fn explore_location_need_driven_priority_tracks_underlying_need_band() {
        let agent = entity(1);
        let target_place = entity(10);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                Permille::new(650).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        );
        view.exploration_profiles.insert(
            agent,
            worldwake_core::ExplorationProfile {
                curiosity_weight: Permille::new(500).unwrap(),
                ..worldwake_core::ExplorationProfile::default()
            },
        );

        let ranked = rank(
            &[goal(GoalKind::ExploreLocation {
                target_place,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    worldwake_core::HomeostaticNeedId::Hunger,
                ),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
    }

    #[test]
    fn explore_location_motive_uses_need_utility_scaled_by_curiosity() {
        let agent = entity(1);
        let target_place = entity(10);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                Permille::new(700).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        );
        view.exploration_profiles.insert(
            agent,
            worldwake_core::ExplorationProfile {
                curiosity_weight: Permille::new(600).unwrap(),
                ..worldwake_core::ExplorationProfile::default()
            },
        );

        let ranked = rank(
            &[goal(GoalKind::ExploreLocation {
                target_place,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    worldwake_core::HomeostaticNeedId::Hunger,
                ),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].motive_score, 378_000);
    }

    #[test]
    fn critical_hunger_exploration_outranks_lower_class_sleep() {
        let agent = entity(1);
        let target_place = entity(10);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                Permille::new(900).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(650).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        );
        view.exploration_profiles.insert(
            agent,
            worldwake_core::ExplorationProfile {
                curiosity_weight: Permille::new(600).unwrap(),
                ..worldwake_core::ExplorationProfile::default()
            },
        );

        let ranked = rank(
            &[
                goal(GoalKind::Sleep),
                goal(GoalKind::ExploreLocation {
                    target_place,
                    motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                        worldwake_core::HomeostaticNeedId::Hunger,
                    ),
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
        )
        .into_ranked();

        assert_eq!(
            ranked[0].offer.key.kind,
            GoalKind::ExploreLocation {
                target_place,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    worldwake_core::HomeostaticNeedId::Hunger,
                ),
            }
        );
    }

    #[test]
    fn explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack() {
        let agent = entity(1);
        let target_place = entity(10);
        let mut view = base_view(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                Permille::new(300).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        );
        view.diversification_profiles.insert(
            agent,
            DiversificationProfile {
                base_curiosity: Permille::new(600).unwrap(),
                comfort_threshold: Permille::new(450).unwrap(),
                curiosity_buildup_rate: Permille::new(5).unwrap(),
                exploration_cooldown_ticks: 60,
                familiarity_per_visit: Permille::new(150).unwrap(),
                familiarity_recovery_per_tick: Permille::new(2).unwrap(),
                familiarity_floor: Permille::new(50).unwrap(),
                max_exploration_hops: 3,
            },
        );
        view.last_proactive_exploration_ticks
            .insert(agent, Tick(100));

        let ranked = rank(
            &[goal(GoalKind::ExploreLocation {
                target_place,
                motivating_need: worldwake_core::ExplorationMotivation::Proactive,
            })],
            &view,
            agent,
            Tick(200),
            &utility(),
        )
        .into_ranked();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].motive_score, 210);
    }
}
