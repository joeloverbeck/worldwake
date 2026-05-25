use crate::{
    GoalDispatchKey, PlannedStep, PlannerOpKind, PlannerOpSemantics, PlanningEntityRef,
    PlanningState,
    decision_trace::{
        PrerequisiteExclusionReason, PrerequisiteExclusionTrace, PrerequisiteGuidanceTrace,
    },
    derive_danger_pressure,
    enterprise::{merchant_home_place, restock_gap_at_destination},
    goal_schema::GoalDispatchKeySchemaExt,
    institutional_queries::consulted_office_holder_read_for_record_data,
    pressure::DangerAssessment,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{
    AcquisitionQuantity, ArtifactActionability, ArtifactKind, BountyTarget, CommodityKind,
    CommodityPurpose, EntityId, EpistemicDispositionProfile, EpistemicSubject, ExecutionBudget,
    GoalKey, GoalKind, InstitutionalBeliefRead, LoadUnits, MultiplierPermille, OUTDOOR_RELIEF_TAGS,
    PerceptionSource, Permille, PlaceTag, Quantity, RecordKind, SuccessionLaw, TellTopic, Tick,
    WorkstationTag, belief_confidence,
};
use worldwake_sim::{
    AccuseActionPayload, ActionDef, ActionPayload, AskAboutPersonActionPayload, AskWitnessPayload,
    CombatActionPayload, ConsultRecordActionPayload, DeclareSupportActionPayload,
    EconomicBeliefView, EntityBeliefView, EscortToSafetyActionPayload, FacilityBeliefView,
    InventoryBeliefView, InvestigateActionPayload, LootActionPayload, PostBountyActionPayload,
    PostNoticeActionPayload, PressForceClaimActionPayload, ProfileBeliefView, PunishActionPayload,
    RecipeDefinition, RecipeRegistry, ReportFoundActionPayload, ReportMissingActionPayload,
    SearchPlaceActionPayload, SocialBeliefView, SpatialBeliefView, TellActionPayload,
    TemporalBeliefView, TradeActionPayload, TransportActionPayload,
};
use worldwake_systems::trade_actions::buyer_trade_opening_offer_for_view;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RankedGoalProvenanceFamily {
    Danger,
    Drive,
    EpistemicSensing,
}

pub trait GoalKindPlannerExt {
    fn ranked_goal_provenance_family(&self) -> Option<RankedGoalProvenanceFamily>;
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
    fn target_commodity(&self, recipes: &RecipeRegistry) -> Option<CommodityKind>;
    fn relevant_observed_commodities(
        &self,
        recipes: &RecipeRegistry,
    ) -> Option<BTreeSet<CommodityKind>>;
    fn build_payload_override(
        &self,
        affordance_payload: Option<&ActionPayload>,
        state: &PlanningState<'_>,
        targets: &[EntityId],
        def: &ActionDef,
        semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, GoalPayloadOverrideError>;
    fn is_progress_barrier(&self, step: &PlannedStep) -> bool;
    fn is_satisfied(&self, state: &PlanningState<'_>) -> bool;
    /// Places where this goal can potentially be achieved.
    /// Used by the A* heuristic to guide travel toward goal-relevant locations.
    /// Returns empty if the goal has no spatial preference (heuristic defaults to h=0).
    fn goal_relevant_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
    ) -> Vec<EntityId>;
    fn prerequisite_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
        execution_budget: &ExecutionBudget,
    ) -> Vec<EntityId>;
    /// Whether the given `op_kind` acting on `authoritative_targets` satisfies
    /// this goal's target-binding requirement.
    ///
    /// - Empty `authoritative_targets` → always `true` (planner-only synthetic candidates).
    /// - Auxiliary ops → always `true` (they serve the goal indirectly).
    /// - Terminal ops on exact-bound goals → `true` only if targets contain the
    ///   goal's canonical entity.
    /// - Flexible goals → always `true` regardless of op or targets.
    fn matches_binding(&self, authoritative_targets: &[EntityId], op_kind: PlannerOpKind) -> bool;
    /// Whether the current planning state lawfully permits considering this operator.
    /// Used for stateful root-candidate filtering when binding alone is insufficient.
    fn candidate_is_available(&self, state: &PlanningState<'_>, op_kind: PlannerOpKind) -> bool;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GoalPayloadOverrideError {
    MissingTarget,
    UnsupportedGoal,
    MissingActorPlace,
    SellerUnavailable,
    SellerOutOfStock,
    ActorCannotPay,
    UnsupportedTopic,
}

fn ask_witness_payload_for_topic(
    witness: EntityId,
    topic: TellTopic,
) -> Result<AskWitnessPayload, GoalPayloadOverrideError> {
    match topic {
        TellTopic::EntityBelief { subject } => Ok(AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        }),
        TellTopic::SocialObservation { .. } | TellTopic::InstitutionalClaim { .. } => {
            Err(GoalPayloadOverrideError::UnsupportedTopic)
        }
    }
}

fn topic_entity_subject(topic: TellTopic) -> Option<EntityId> {
    match topic {
        TellTopic::EntityBelief { subject } => Some(subject),
        TellTopic::SocialObservation { .. } | TellTopic::InstitutionalClaim { .. } => None,
    }
}

fn ask_witness_payload_matches_subject(
    payload: &AskWitnessPayload,
    subject: EpistemicSubject,
) -> bool {
    match subject {
        EpistemicSubject::EntityLocation { entity, .. } => payload.topic_entity == Some(entity),
        EpistemicSubject::SupplyAvailability {
            commodity, source, ..
        } => payload.topic_entity == Some(source) && payload.topic_commodity == Some(commodity),
    }
}

fn report_is_fresh_enough_for_witness_preference(
    staleness_ticks: u64,
    profile: &EpistemicDispositionProfile,
    confidence_policy: &worldwake_core::BeliefConfidencePolicy,
) -> bool {
    let staleness_penalty = u64::from(confidence_policy.staleness_penalty_per_tick.value());
    let freshness_budget = u64::from(profile.witness_recency_preference.value());
    staleness_ticks.saturating_mul(staleness_penalty) <= freshness_budget
}

pub(crate) fn epistemic_subject_for_belief(
    entity: EntityId,
    belief: &worldwake_core::BelievedEntityState,
) -> Option<EpistemicSubject> {
    let place = belief.last_known_place?;
    belief
        .resource_source
        .as_ref()
        .map(|resource| EpistemicSubject::SupplyAvailability {
            commodity: resource.commodity,
            source: entity,
            place,
        })
        .or(Some(EpistemicSubject::EntityLocation { entity, place }))
}

pub(crate) fn grounded_goal_epistemic_subjects(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
) -> Vec<EpistemicSubject> {
    if matches!(goal.key.kind, GoalKind::Accuse { .. }) {
        return Vec::new();
    }
    let actor = state.snapshot().actor();
    let Some(profile) = state.epistemic_disposition_profile(actor) else {
        return Vec::new();
    };
    let policy = state.belief_confidence_policy(actor);
    let current_tick = state.current_tick();

    goal.evidence_entities
        .iter()
        .filter_map(|entity| {
            if *entity == actor {
                return None;
            }
            let belief = state
                .known_entity_beliefs(actor)
                .into_iter()
                .find_map(|(known, belief)| (known == *entity).then_some(belief))?;
            let staleness_ticks = current_tick
                .0
                .saturating_sub(belief.last_observed_tick().unwrap_or(Tick(0)).0);
            if belief_confidence(&belief.source, staleness_ticks, &policy)
                >= profile.stale_evidence_barrier_threshold
            {
                return None;
            }
            let subject = epistemic_subject_for_belief(*entity, &belief)?;
            let place = match subject {
                EpistemicSubject::EntityLocation { place, .. }
                | EpistemicSubject::SupplyAvailability { place, .. } => place,
            };
            let anchored_here = matches!(
                goal.anchor,
                worldwake_core::OpportunityAnchor::Place(anchor) if anchor == place
            );
            (anchored_here || goal.evidence_places.contains(&place)).then_some(subject)
        })
        .collect()
}

pub(crate) fn grounded_goal_matches_epistemic_barrier(
    subjects: &[EpistemicSubject],
    op_kind: PlannerOpKind,
    authoritative_targets: &[EntityId],
    payload: Option<&ActionPayload>,
) -> bool {
    if subjects.is_empty() {
        return false;
    }

    match (op_kind, payload) {
        (PlannerOpKind::Travel, _) => subjects.iter().any(|subject| match subject {
            EpistemicSubject::EntityLocation { place, .. }
            | EpistemicSubject::SupplyAvailability { place, .. } => {
                authoritative_targets.contains(place)
            }
        }),
        (PlannerOpKind::AskWitness, Some(ActionPayload::AskWitness(ask))) => subjects
            .iter()
            .any(|subject| ask_witness_payload_matches_subject(ask, *subject)),
        _ => false,
    }
}

pub(crate) fn grounded_goal_allows_local_epistemic_resolution(
    goal: &GoalOffer,
    op_kind: PlannerOpKind,
    authoritative_targets: &[EntityId],
) -> bool {
    match (&goal.key.kind, op_kind) {
        (GoalKind::InvestigateViolation { place, .. }, PlannerOpKind::Investigate) => {
            authoritative_targets.contains(place)
        }
        _ => false,
    }
}

fn payload_override_from_affordance(
    goal: &GoalKind,
    affordance_payload: Option<&ActionPayload>,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    let Some(payload) = affordance_payload else {
        return Ok(None);
    };

    match goal {
        GoalKind::EngageHostile { target } | GoalKind::RaidTarget { target } => payload
            .as_combat()
            .filter(|combat| combat.target == *target)
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        GoalKind::ShareBelief {
            listener, topic, ..
        } => payload
            .as_tell()
            .filter(|tell| tell.listener == *listener && tell.topic == *topic)
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        GoalKind::AskWitness { witness, topic } => payload
            .as_ask_witness()
            .filter(|ask| {
                ask_witness_payload_for_topic(*witness, *topic)
                    .is_ok_and(|expected| **ask == expected)
            })
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        GoalKind::InvestigateViolation { violation_id, .. } => payload
            .as_investigate()
            .filter(|investigate| investigate.violation_id == *violation_id)
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        GoalKind::Accuse { violation_id, .. } => payload
            .as_accuse()
            .filter(|accuse| accuse.violation_id == *violation_id)
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        _ => Ok(Some(payload.clone())),
    }
}

fn build_attack_payload_override(
    goal: &GoalKind,
    targets: &[EntityId],
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::EngageHostile { target } | GoalKind::RaidTarget { target } => {
            let Some(actual_target) = targets.first().copied() else {
                return Err(GoalPayloadOverrideError::MissingTarget);
            };
            if actual_target != *target {
                return Err(GoalPayloadOverrideError::UnsupportedGoal);
            }
            Ok(Some(ActionPayload::Combat(CombatActionPayload {
                target: actual_target,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            })))
        }
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_search_place_payload_override(
    goal: &GoalKind,
    state: &PlanningState<'_>,
    targets: &[EntityId],
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    let GoalKind::SearchForMissing { subject, .. } = goal else {
        return Err(GoalPayloadOverrideError::UnsupportedGoal);
    };
    let actor = state.snapshot().actor();
    let actor_place = state
        .effective_place(actor)
        .ok_or(GoalPayloadOverrideError::MissingActorPlace)?;
    let Some(target_place) = targets.first().copied() else {
        return Err(GoalPayloadOverrideError::MissingTarget);
    };
    if target_place != actor_place {
        return Err(GoalPayloadOverrideError::UnsupportedGoal);
    }
    Ok(Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
        subject: *subject,
    })))
}

fn build_declare_support_payload_override(
    goal: &GoalKind,
    actor: EntityId,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::ClaimOffice { office } => Ok(Some(ActionPayload::DeclareSupport(
            DeclareSupportActionPayload {
                office: *office,
                candidate: actor,
            },
        ))),
        GoalKind::SupportCandidateForOffice { office, candidate } => Ok(Some(
            ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office: *office,
                candidate: *candidate,
            }),
        )),
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_press_force_claim_payload_override(
    goal: &GoalKind,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::ClaimOffice { office } => Ok(Some(ActionPayload::PressForceClaim(
            PressForceClaimActionPayload { office: *office },
        ))),
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_loot_payload_override(
    targets: &[EntityId],
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    let Some(target) = targets.first().copied() else {
        return Err(GoalPayloadOverrideError::MissingTarget);
    };
    Ok(Some(ActionPayload::Loot(LootActionPayload { target })))
}

fn build_accuse_payload_override(
    goal: &GoalKind,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::Accuse { violation_id, .. } => {
            Ok(Some(ActionPayload::Accuse(AccuseActionPayload {
                violation_id: *violation_id,
            })))
        }
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_punish_payload_override(
    goal: &GoalKind,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::PunishAccused {
            office,
            accusation_entry,
            punishment,
            ..
        } => Ok(Some(ActionPayload::Punish(PunishActionPayload {
            office: *office,
            accusation_entry: *accusation_entry,
            punishment: *punishment,
        }))),
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn office_requiring_vacancy_belief(goal: &GoalKind) -> Option<EntityId> {
    match goal {
        GoalKind::ClaimOffice { office } | GoalKind::SupportCandidateForOffice { office, .. } => {
            Some(*office)
        }
        _ => None,
    }
}

fn office_succession_law(state: &PlanningState<'_>, office: EntityId) -> Option<SuccessionLaw> {
    state.succession_law(office)
}

fn political_step_requires_known_vacancy(goal: &GoalKind, op_kind: PlannerOpKind) -> bool {
    matches!(
        (goal, op_kind),
        (GoalKind::ClaimOffice { .. }, PlannerOpKind::Bribe)
            | (GoalKind::ClaimOffice { .. }, PlannerOpKind::Threaten)
            | (
                GoalKind::ClaimOffice { .. } | GoalKind::SupportCandidateForOffice { .. },
                PlannerOpKind::DeclareSupport,
            )
    )
}

fn consulted_office_holder_read_for_record(
    state: &PlanningState<'_>,
    record: EntityId,
    office: EntityId,
) -> InstitutionalBeliefRead<Option<EntityId>> {
    state
        .record_data(record)
        .as_ref()
        .map_or(InstitutionalBeliefRead::Unknown, |record_data| {
            consulted_office_holder_read_for_record_data(record_data, office)
        })
}

fn office_register_for_goal(
    state: &PlanningState<'_>,
    office: EntityId,
) -> Option<(
    EntityId,
    EntityId,
    InstitutionalBeliefRead<Option<EntityId>>,
)> {
    state
        .snapshot()
        .entities
        .iter()
        .filter_map(|(&entity, snapshot)| {
            let record_data = snapshot.political.record_data.as_ref()?;
            (record_data.record_kind == RecordKind::OfficeRegister).then_some((
                entity,
                record_data.home_place,
                consulted_office_holder_read_for_record(state, entity, office),
            ))
        })
        .find(|(_, _, read)| !matches!(read, InstitutionalBeliefRead::Unknown))
}

fn political_step_blocked_by_unknown_vacancy(
    goal: &GoalKind,
    state: &PlanningState<'_>,
    op_kind: PlannerOpKind,
) -> bool {
    if !political_step_requires_known_vacancy(goal, op_kind) {
        return false;
    }
    let Some(office) = office_requiring_vacancy_belief(goal) else {
        return true;
    };
    office_register_for_goal(state, office).is_some()
        && state.believed_office_holder(office) != InstitutionalBeliefRead::Certain(None)
}

fn believed_bounty_terms(
    state: &PlanningState<'_>,
    bounty: EntityId,
) -> Option<worldwake_core::BelievedBountyTerms> {
    let actor = state.snapshot().actor();
    state
        .known_entity_beliefs(actor)
        .into_iter()
        .find_map(|(entity, belief)| {
            (entity == bounty)
                .then(|| {
                    belief
                        .believed_artifact
                        .and_then(|artifact| artifact.bounty_terms)
                })
                .flatten()
        })
}

fn believed_bounty_artifact_state(
    state: &PlanningState<'_>,
    bounty: EntityId,
) -> Option<worldwake_core::BelievedArtifactState> {
    let actor = state.snapshot().actor();
    state
        .known_entity_beliefs(actor)
        .into_iter()
        .find_map(|(entity, belief)| (entity == bounty).then_some(belief.believed_artifact))
        .flatten()
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct FreeCarryCapacityContract {
    pub(crate) current_load: LoadUnits,
    pub(crate) carry_capacity: LoadUnits,
    pub(crate) disposal_threshold: Permille,
    pub(crate) has_waste_targets: bool,
}

impl FreeCarryCapacityContract {
    pub(crate) fn new(
        current_load: LoadUnits,
        carry_capacity: LoadUnits,
        disposal_threshold: Permille,
        has_waste_targets: bool,
    ) -> Self {
        Self {
            current_load,
            carry_capacity,
            disposal_threshold,
            has_waste_targets,
        }
    }

    fn is_below_threshold(self) -> bool {
        self.current_load.0.saturating_mul(1_000)
            < self
                .carry_capacity
                .0
                .saturating_mul(u32::from(self.disposal_threshold.value()))
    }

    pub(crate) fn is_actionable(self) -> bool {
        self.has_waste_targets && !self.is_below_threshold()
    }

    pub(crate) fn is_satisfied(self, root_baseline_load: Option<LoadUnits>) -> bool {
        if !self.is_actionable() {
            return true;
        }

        root_baseline_load.is_some_and(|baseline_load| {
            self.current_load.0 < baseline_load.0 && self.is_below_threshold()
        })
    }
}

pub(crate) fn free_carry_capacity_contract_from_view(
    view: &dyn worldwake_sim::GoalBeliefView,
    agent: EntityId,
) -> Option<FreeCarryCapacityContract> {
    let carry_capacity = view.carry_capacity(agent)?;
    let current_load = view
        .direct_possessions(agent)
        .into_iter()
        .try_fold(0u32, |total, entity| {
            view.load_of_entity(entity)
                .and_then(|load| total.checked_add(load.0))
        })
        .map(LoadUnits)?;
    let disposal_threshold = view
        .disposal_profile(agent)
        .map_or(Permille::new_unchecked(800), |profile| {
            profile.capacity_strain_threshold
        });
    let has_waste_targets = view
        .known_entity_beliefs(agent)
        .into_iter()
        .any(|(item, belief)| {
            belief.believed_kind == Some(worldwake_core::EntityKind::ItemLot)
                && view.direct_possessor(item) == Some(agent)
                && belief
                    .last_known_inventory
                    .get(&CommodityKind::Waste)
                    .is_some_and(|quantity| *quantity > Quantity(0))
        });

    Some(FreeCarryCapacityContract::new(
        current_load,
        carry_capacity,
        disposal_threshold,
        has_waste_targets,
    ))
}

impl GoalKindPlannerExt for GoalKind {
    fn ranked_goal_provenance_family(&self) -> Option<RankedGoalProvenanceFamily> {
        GoalDispatchKey::from_goal_kind(self)
            .declaration()
            .provenance_family
    }

    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind] {
        GoalDispatchKey::from_goal_kind(self)
            .declaration()
            .relevant_ops
    }

    fn target_commodity(&self, recipes: &RecipeRegistry) -> Option<CommodityKind> {
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity }
            | GoalKind::MoveCargo { commodity, .. } => Some(*commodity),
            GoalKind::TreatWounds { .. } => Some(CommodityKind::Medicine),
            GoalKind::ProduceCommodity { recipe_id } => recipes
                .get(*recipe_id)
                .and_then(|recipe| recipe.outputs.first().map(|(commodity, _)| *commodity)),
            GoalKind::FreeCarryCapacity => Some(CommodityKind::Waste),
            GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::EngageHostile { .. }
            | GoalKind::RaidTarget { .. }
            | GoalKind::ReduceDanger
            | GoalKind::RegroupWithFaction { .. }
            | GoalKind::EstablishBanditCamp { .. }
            | GoalKind::SearchForMissing { .. }
            | GoalKind::ReportMissing { .. }
            | GoalKind::EscortToSafety { .. }
            | GoalKind::LootCorpse { .. }
            | GoalKind::BuryCorpse { .. }
            | GoalKind::FulfillBounty { .. }
            | GoalKind::PostBounty { .. }
            | GoalKind::PostNotice { .. }
            | GoalKind::ShareBelief { .. }
            | GoalKind::AskWitness { .. }
            | GoalKind::ClaimOffice { .. }
            | GoalKind::SupportCandidateForOffice { .. }
            | GoalKind::InvestigateViolation { .. }
            | GoalKind::Patrol { .. }
            | GoalKind::ExploreLocation { .. }
            | GoalKind::StealItem { .. }
            | GoalKind::Accuse { .. }
            | GoalKind::PunishAccused { .. }
            | GoalKind::ReportFound { .. } => None,
        }
    }

    fn relevant_observed_commodities(
        &self,
        recipes: &RecipeRegistry,
    ) -> Option<BTreeSet<CommodityKind>> {
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity }
            | GoalKind::MoveCargo { commodity, .. } => Some([*commodity].into_iter().collect()),
            GoalKind::ProduceCommodity { recipe_id } => recipes.get(*recipe_id).map(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .chain(recipe.outputs.iter())
                    .map(|(commodity, _)| *commodity)
                    .collect()
            }),
            GoalKind::FreeCarryCapacity => Some(BTreeSet::from([CommodityKind::Waste])),
            GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::EngageHostile { .. }
            | GoalKind::RaidTarget { .. }
            | GoalKind::ReduceDanger
            | GoalKind::RegroupWithFaction { .. }
            | GoalKind::EstablishBanditCamp { .. }
            | GoalKind::TreatWounds { .. }
            | GoalKind::SearchForMissing { .. }
            | GoalKind::ReportMissing { .. }
            | GoalKind::EscortToSafety { .. }
            | GoalKind::LootCorpse { .. }
            | GoalKind::BuryCorpse { .. }
            | GoalKind::FulfillBounty { .. }
            | GoalKind::PostBounty { .. }
            | GoalKind::PostNotice { .. }
            | GoalKind::ShareBelief { .. }
            | GoalKind::AskWitness { .. }
            | GoalKind::ClaimOffice { .. }
            | GoalKind::SupportCandidateForOffice { .. }
            | GoalKind::InvestigateViolation { .. }
            | GoalKind::Patrol { .. }
            | GoalKind::ExploreLocation { .. }
            | GoalKind::StealItem { .. }
            | GoalKind::Accuse { .. }
            | GoalKind::PunishAccused { .. }
            | GoalKind::ReportFound { .. } => Some(BTreeSet::new()),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn build_payload_override(
        &self,
        affordance_payload: Option<&ActionPayload>,
        state: &PlanningState<'_>,
        targets: &[EntityId],
        def: &ActionDef,
        semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
        if let GoalKind::FulfillBounty { bounty } = self
            && matches!(
                semantics.op_kind,
                PlannerOpKind::Attack | PlannerOpKind::ClaimBounty
            )
        {
            let Some(terms) = believed_bounty_terms(state, *bounty) else {
                return Err(GoalPayloadOverrideError::UnsupportedGoal);
            };
            match (semantics.op_kind, terms.target) {
                (PlannerOpKind::Attack, BountyTarget::EliminateEntity { target }) => {
                    let Some(actual_target) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if actual_target != target || state.is_dead(target) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    return Ok(Some(ActionPayload::Combat(CombatActionPayload {
                        target: actual_target,
                        weapon: worldwake_core::CombatWeaponRef::Unarmed,
                    })));
                }
                (PlannerOpKind::ClaimBounty, BountyTarget::EliminateEntity { target }) => {
                    if !state.is_dead(target) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                }
                _ => {}
            }
        }

        let actor = state.snapshot().actor();
        if let Some(payload) = payload_override_from_affordance(self, affordance_payload)? {
            if semantics.op_kind == PlannerOpKind::ConsultRecord {
                let Some(record) = payload.as_consult_record().map(|consult| consult.record) else {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                };
                let Some(office) = office_requiring_vacancy_belief(self) else {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                };
                if matches!(
                    consulted_office_holder_read_for_record(state, record, office),
                    InstitutionalBeliefRead::Unknown
                ) {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
            }
            if let Some(ask) = payload.as_ask_witness() {
                let cooldown_key = worldwake_core::AskWitnessMemoryKey {
                    counterparty: ask.target,
                    topic_entity: ask.topic_entity,
                    topic_commodity: ask.topic_commodity,
                };
                if state.ask_witness_memory(actor, &cooldown_key).is_some() {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
            }
            if let Some(ask) = payload.as_ask_about_person() {
                let cooldown_key = worldwake_core::AskWitnessMemoryKey {
                    counterparty: ask.target,
                    topic_entity: Some(ask.subject),
                    topic_commodity: None,
                };
                if state.ask_witness_memory(actor, &cooldown_key).is_some() {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
            }
            return Ok(Some(payload));
        }

        if political_step_blocked_by_unknown_vacancy(self, state, semantics.op_kind) {
            return Err(GoalPayloadOverrideError::UnsupportedGoal);
        }
        match semantics.op_kind {
            PlannerOpKind::ConsultRecord => {
                let Some(office) = office_requiring_vacancy_belief(self) else {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                };
                let Some(record) = targets.first().copied() else {
                    return Err(GoalPayloadOverrideError::MissingTarget);
                };
                if matches!(
                    consulted_office_holder_read_for_record(state, record, office),
                    InstitutionalBeliefRead::Unknown
                ) {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
                Ok(Some(ActionPayload::ConsultRecord(
                    ConsultRecordActionPayload { record },
                )))
            }
            PlannerOpKind::Patrol => Ok(None),
            PlannerOpKind::Trade => {
                let Some(counterparty) = targets.first().copied() else {
                    return Err(GoalPayloadOverrideError::MissingTarget);
                };
                let requested_commodity = match self {
                    GoalKind::AcquireCommodity { commodity, .. }
                    | GoalKind::RestockCommodity { commodity }
                    | GoalKind::ConsumeOwnedCommodity { commodity } => *commodity,
                    GoalKind::TreatWounds { .. } => CommodityKind::Medicine,
                    _ => return Err(GoalPayloadOverrideError::UnsupportedGoal),
                };
                let Some(actor_place) = state.effective_place(actor) else {
                    return Err(GoalPayloadOverrideError::MissingActorPlace);
                };
                // Find a concrete listed sale lot owned by the counterparty.
                let sale_lot = state
                    .listed_sale_lots_at(actor_place, requested_commodity)
                    .into_iter()
                    .find(|lot| state.seller_for_sale_lot(*lot) == Some(counterparty));
                let Some(sale_lot) = sale_lot else {
                    return Err(GoalPayloadOverrideError::SellerUnavailable);
                };
                if state.commodity_quantity(counterparty, requested_commodity) == Quantity(0) {
                    return Err(GoalPayloadOverrideError::SellerOutOfStock);
                }
                if state.commodity_quantity(actor, CommodityKind::Coin) == Quantity(0) {
                    return Err(GoalPayloadOverrideError::ActorCannotPay);
                }
                let Some(offered_quantity) = buyer_trade_opening_offer_for_view(
                    state,
                    actor,
                    counterparty,
                    actor_place,
                    requested_commodity,
                ) else {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                };
                Ok(Some(ActionPayload::Trade(TradeActionPayload {
                    counterparty,
                    sale_lot,
                    offered_commodity: CommodityKind::Coin,
                    offered_quantity,
                    requested_quantity: Quantity(1),
                })))
            }
            PlannerOpKind::Harvest => {
                let Some(workstation) = targets.first().copied() else {
                    return Err(GoalPayloadOverrideError::MissingTarget);
                };
                let Some(actor_place) = state.effective_place(actor) else {
                    return Err(GoalPayloadOverrideError::MissingActorPlace);
                };
                if state.effective_place(workstation) != Some(actor_place) {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
                let Some(payload) = def.payload.as_harvest() else {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                };
                if state.workstation_tag(workstation) != Some(payload.required_workstation_tag) {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
                let requested_commodity = match self {
                    GoalKind::AcquireCommodity { commodity, .. }
                    | GoalKind::RestockCommodity { commodity }
                    | GoalKind::ConsumeOwnedCommodity { commodity } => *commodity,
                    GoalKind::TreatWounds { .. } => CommodityKind::Medicine,
                    _ => return Err(GoalPayloadOverrideError::UnsupportedGoal),
                };
                if payload.output_commodity != requested_commodity {
                    return Err(GoalPayloadOverrideError::UnsupportedGoal);
                }
                Ok(Some(def.payload.clone()))
            }
            PlannerOpKind::Investigate => match self {
                GoalKind::InvestigateViolation { violation_id, .. } => {
                    Ok(Some(ActionPayload::Investigate(InvestigateActionPayload {
                        violation_id: *violation_id,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::EstablishCamp => match self {
                GoalKind::EstablishBanditCamp { faction } => {
                    Ok(Some(ActionPayload::EstablishCamp(
                        worldwake_sim::EstablishCampActionPayload { faction: *faction },
                    )))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::AskWitness => match self {
                GoalKind::AskWitness { witness, topic } => {
                    let actor = state.snapshot().actor();
                    let Some(target_witness) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if target_witness != *witness {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    let payload = ask_witness_payload_for_topic(*witness, *topic)?;
                    let cooldown_key = worldwake_core::AskWitnessMemoryKey {
                        counterparty: payload.target,
                        topic_entity: payload.topic_entity,
                        topic_commodity: payload.topic_commodity,
                    };
                    if state.ask_witness_memory(actor, &cooldown_key).is_some() {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::AskWitness(payload)))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::ReportFound => match self {
                GoalKind::ReportFound { expectation_id, .. } => {
                    let Some(&target) = targets.first() else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    Ok(Some(ActionPayload::ReportFound(ReportFoundActionPayload {
                        target,
                        expectation_id: *expectation_id,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::AskAboutPerson => match self {
                GoalKind::SearchForMissing { subject, last_seen } => {
                    let actor = state.snapshot().actor();
                    if state
                        .effective_place(actor)
                        .is_some_and(|actor_place| Some(actor_place) == *last_seen)
                    {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    let Some(&target) = targets.first() else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let cooldown_key = worldwake_core::AskWitnessMemoryKey {
                        counterparty: target,
                        topic_entity: Some(*subject),
                        topic_commodity: None,
                    };
                    if state.ask_witness_memory(actor, &cooldown_key).is_some() {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::AskAboutPerson(
                        AskAboutPersonActionPayload {
                            target,
                            subject: *subject,
                        },
                    )))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::EscortToSafety => match self {
                GoalKind::EscortToSafety {
                    subject,
                    destination,
                } => {
                    // Candidate generation already verified wound state; the
                    // planner only needs a plausible payload.  Route fields are
                    // filled at action start, not during planning.
                    Ok(Some(ActionPayload::EscortToSafety(
                        EscortToSafetyActionPayload {
                            subject: *subject,
                            destination: *destination,
                            intended_heal_action: None,
                            route_places: Vec::new(),
                            route_edges: Vec::new(),
                        },
                    )))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::ReportMissing => match self {
                GoalKind::ReportMissing {
                    expectation_id: Some(expectation_id),
                    ..
                } => Ok(Some(ActionPayload::ReportMissing(
                    ReportMissingActionPayload {
                        expectation_id: *expectation_id,
                    },
                ))),
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::SearchPlace => build_search_place_payload_override(self, state, targets),
            PlannerOpKind::Accuse => build_accuse_payload_override(self),
            PlannerOpKind::Fine | PlannerOpKind::Exile => build_punish_payload_override(self),
            PlannerOpKind::Attack => build_attack_payload_override(self, targets),
            PlannerOpKind::Tell => match self {
                GoalKind::ShareBelief {
                    listener, topic, ..
                } => {
                    let Some(target_listener) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if target_listener != *listener {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::Tell(TellActionPayload {
                        listener: *listener,
                        topic: *topic,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::DeclareSupport => match self {
                GoalKind::ClaimOffice { office }
                    if office_succession_law(state, *office) != Some(SuccessionLaw::Support) =>
                {
                    Err(GoalPayloadOverrideError::UnsupportedGoal)
                }
                GoalKind::ClaimOffice { .. } | GoalKind::SupportCandidateForOffice { .. } => {
                    build_declare_support_payload_override(self, actor)
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::PressForceClaim => match self {
                GoalKind::ClaimOffice { office }
                    if office_succession_law(state, *office) == Some(SuccessionLaw::Force) =>
                {
                    build_press_force_claim_payload_override(self)
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::StaffMarket => match self {
                GoalKind::SellCommodity { commodity } => Ok(Some(ActionPayload::StaffMarket(
                    worldwake_sim::StaffMarketPayload {
                        commodity: *commodity,
                    },
                ))),
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::Loot => build_loot_payload_override(targets),
            PlannerOpKind::PostBounty => match self {
                GoalKind::PostBounty { posting, terms } => {
                    Ok(Some(ActionPayload::PostBounty(PostBountyActionPayload {
                        posting_place: posting.posting_place,
                        issuing_authority: posting.issuing_authority,
                        expires_at: posting.expires_at,
                        jurisdiction: posting.jurisdiction,
                        target: terms.target,
                        proof_requirement: terms.proof_requirement,
                        reward_commodity: terms.reward_commodity,
                        reward_quantity: terms.reward_quantity,
                        reward_source: terms.reward_source,
                        claim_place: terms.claim_place,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::PostNotice => match self {
                GoalKind::PostNotice { posting, topic } => {
                    Ok(Some(ActionPayload::PostNotice(PostNoticeActionPayload {
                        posting_place: posting.posting_place,
                        issuing_authority: posting.issuing_authority,
                        expires_at: posting.expires_at,
                        jurisdiction: posting.jurisdiction,
                        topic: *topic,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::MoveCargo => match self {
                GoalKind::MoveCargo {
                    commodity,
                    destination,
                } if def.name == "pick_up" => {
                    let Some(target) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if state.item_lot_commodity(target) != Some(*commodity) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    let lot_quantity = state.commodity_quantity(target, *commodity);
                    let Some(restock_gap) =
                        restock_gap_at_destination(state, actor, *destination, *commodity)
                    else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let remaining_capacity = state
                        .remaining_carry_capacity_ref(crate::PlanningEntityRef::Authoritative(
                            actor,
                        ))
                        .ok_or(GoalPayloadOverrideError::UnsupportedGoal)?
                        .0;
                    let per_unit = worldwake_core::load_per_unit(*commodity).0;
                    let carry_fit = Quantity(remaining_capacity / per_unit);
                    let quantity = Quantity(lot_quantity.0.min(restock_gap.0).min(carry_fit.0));
                    if quantity == Quantity(0) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::Transport(TransportActionPayload {
                        quantity,
                    })))
                }
                GoalKind::FulfillBounty { bounty } if def.name == "pick_up" => {
                    let Some(terms) = believed_bounty_terms(state, *bounty) else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let BountyTarget::DeliverCommodity {
                        commodity,
                        quantity,
                        destination,
                    } = terms.target
                    else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let Some(target) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if state.item_lot_commodity(target) != Some(commodity) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    let lot_quantity = state.commodity_quantity(target, commodity);
                    let Some(delivery_gap) = delivery_bounty_gap_at_destination(
                        state,
                        actor,
                        destination,
                        commodity,
                        quantity,
                    ) else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let remaining_capacity = state
                        .remaining_carry_capacity_ref(crate::PlanningEntityRef::Authoritative(
                            actor,
                        ))
                        .ok_or(GoalPayloadOverrideError::UnsupportedGoal)?
                        .0;
                    let per_unit = worldwake_core::load_per_unit(commodity).0;
                    let carry_fit = Quantity(remaining_capacity / per_unit);
                    let quantity = Quantity(lot_quantity.0.min(delivery_gap.0).min(carry_fit.0));
                    if quantity == Quantity(0) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::Transport(TransportActionPayload {
                        quantity,
                    })))
                }
                _ => Ok((!matches!(def.payload, ActionPayload::None)).then(|| def.payload.clone())),
            },
            _ => Ok((!matches!(def.payload, ActionPayload::None)).then(|| def.payload.clone())),
        }
    }

    fn is_progress_barrier(&self, step: &PlannedStep) -> bool {
        if step.op_kind == PlannerOpKind::QueueForFacilityUse {
            return matches!(
                self,
                GoalKind::ConsumeOwnedCommodity { .. }
                    | GoalKind::AcquireCommodity { .. }
                    | GoalKind::LootCorpse { .. }
                    | GoalKind::BuryCorpse { .. }
                    | GoalKind::TreatWounds { .. }
                    | GoalKind::ProduceCommodity { .. }
                    | GoalKind::RestockCommodity { .. }
            );
        }

        if matches!(self, GoalKind::ReportFound { .. })
            && step.op_kind == PlannerOpKind::ReportFound
        {
            return true;
        }

        // Direct per-goal op_kind barriers — delegated to the declaration table.
        let decl = crate::GoalDispatchKey::from_goal_kind(self).declaration();
        if !decl.progress_barrier_ops.is_empty()
            && decl.progress_barrier_ops.contains(&step.op_kind)
        {
            return true;
        }

        // ConsumeOwnedCommodity treats pick_up (MoveCargo) as a progress barrier
        // because the planner cannot model possession transfer in hypothetical state.
        // This check runs before the is_materialization_barrier guard because MoveCargo
        // is not a materialization barrier but IS a logical barrier for consumption goals.
        if matches!(self, GoalKind::ConsumeOwnedCommodity { .. })
            && step.op_kind == PlannerOpKind::MoveCargo
        {
            return true;
        }

        if !step.is_materialization_barrier {
            return false;
        }

        // Cargo state changes are modeled by transport transition kinds in planner_ops.rs, and
        // the commodity+destination goal identity survives lot splitting, so cargo intentionally
        // falls through the default non-barrier behavior here.
        match self {
            GoalKind::AcquireCommodity { .. }
            | GoalKind::ProduceCommodity { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::LootCorpse { .. }
            | GoalKind::BuryCorpse { .. } => true,
            GoalKind::TreatWounds { .. } => {
                matches!(
                    step.op_kind,
                    PlannerOpKind::Trade | PlannerOpKind::QueueForFacilityUse
                )
            }
            _ => false,
        }
    }

    fn is_satisfied(&self, state: &PlanningState<'_>) -> bool {
        let actor = state.snapshot().actor();
        let direct_possession_quantity = |commodity: CommodityKind| -> u32 {
            state
                .direct_possessions_ref(PlanningEntityRef::Authoritative(actor))
                .into_iter()
                .filter(|entity| state.item_lot_commodity_ref(*entity) == Some(commodity))
                .map(|entity| state.commodity_quantity_ref(entity, commodity).0)
                .sum()
        };
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity } => {
                let Some(needs) = state.homeostatic_needs(actor) else {
                    return false;
                };
                let Some(thresholds) = state.drive_thresholds(actor) else {
                    return false;
                };
                commodity.spec().consumable_profile.is_some_and(|profile| {
                    let relieves_hunger = profile.hunger_relief_per_unit.value() > 0
                        && needs.hunger >= thresholds.hunger.low();
                    let relieves_thirst = profile.thirst_relief_per_unit.value() > 0
                        && needs.thirst >= thresholds.thirst.low();
                    !(relieves_hunger || relieves_thirst)
                })
            }
            GoalKind::AcquireCommodity {
                commodity,
                purpose,
                quantity,
            } => {
                let floor = u32::from(quantity.desired_min.get());
                match purpose {
                    CommodityPurpose::SelfConsume => {
                        direct_possession_quantity(*commodity) >= floor
                    }
                    CommodityPurpose::Restock | CommodityPurpose::RecipeInput(_) => {
                        state.commodity_quantity(actor, *commodity).0 >= floor
                    }
                }
            }
            GoalKind::Sleep => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.fatigue < thresholds.fatigue.low()),
            GoalKind::Relieve => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.bladder < thresholds.bladder.low()),
            GoalKind::Wash => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.dirtiness < thresholds.dirtiness.low()),
            GoalKind::EngageHostile { target } | GoalKind::RaidTarget { target } => {
                state.is_dead(*target)
            }
            GoalKind::ReduceDanger => state.drive_thresholds(actor).is_some_and(|thresholds| {
                derive_danger_pressure(state, actor) < thresholds.danger.high()
            }),
            GoalKind::RegroupWithFaction { faction } => matches!(
                state.believed_faction_rally_point(*faction),
                InstitutionalBeliefRead::Certain(Some(rally_place))
                    if state.effective_place(actor) == Some(rally_place)
            ),
            GoalKind::EstablishBanditCamp { faction } => {
                state
                    .effective_place(actor)
                    .and_then(|place| state.bandit_camp_faction_at(place))
                    == Some(*faction)
            }
            GoalKind::TreatWounds { patient } => state
                .pain_summary(*patient)
                .is_some_and(|pain| pain == Permille::new_unchecked(0)),
            GoalKind::MoveCargo {
                commodity,
                destination,
            } => restock_gap_at_destination(state, actor, *destination, *commodity).is_none(),
            GoalKind::StealItem { target_item } => {
                state.direct_possessor(*target_item) == Some(actor)
            }
            GoalKind::LootCorpse { corpse } => CommodityKind::ALL
                .iter()
                .copied()
                .all(|commodity| state.commodity_quantity(*corpse, commodity) == Quantity(0)),
            GoalKind::BuryCorpse { corpse, .. } => state.direct_container(*corpse).is_some(),
            GoalKind::FulfillBounty { bounty } => believed_bounty_artifact_state(state, *bounty)
                .is_some_and(|artifact| {
                    artifact.kind == ArtifactKind::Bounty
                        && artifact.actionability != ArtifactActionability::Actionable
                }),
            GoalKind::SupportCandidateForOffice { office, candidate } => {
                state.effective_support_declaration(actor, *office) == Some(*candidate)
            }
            GoalKind::ClaimOffice { office } => match office_succession_law(state, *office) {
                Some(SuccessionLaw::Support) => state.has_support_majority(*office, actor),
                Some(SuccessionLaw::Force) => matches!(
                    state.believed_force_controller(*office),
                    InstitutionalBeliefRead::Certain((Some(controller), false))
                        if controller == actor
                ),
                None => false,
            },
            GoalKind::SellCommodity { commodity } => {
                let home_place = merchant_home_place(state, actor, None);
                let at_market = home_place.is_some() && state.effective_place(actor) == home_place;
                at_market
                    && !state
                        .listed_sale_lots_at(home_place.unwrap(), *commodity)
                        .is_empty()
            }
            GoalKind::EscortToSafety {
                subject,
                destination,
            } => state.effective_place(*subject) == Some(*destination),
            GoalKind::ExploreLocation { target_place, .. } => {
                state.effective_place(actor) == Some(*target_place)
            }
            GoalKind::AskWitness { witness, topic } => {
                ask_witness_goal_satisfied(state, actor, *witness, *topic)
            }
            GoalKind::FreeCarryCapacity => {
                let Some(current_load) = carried_load_of_actor(state, actor) else {
                    return false;
                };
                let Some(carry_capacity) =
                    state.carry_capacity_ref(PlanningEntityRef::Authoritative(actor))
                else {
                    return false;
                };
                let contract = FreeCarryCapacityContract::new(
                    current_load,
                    carry_capacity,
                    state
                        .disposal_profile(actor)
                        .map_or(Permille::new_unchecked(800), |profile| {
                            profile.capacity_strain_threshold
                        }),
                    !free_carry_capacity_drop_targets(state).is_empty(),
                );
                let root_baseline_state = PlanningState::new(state.snapshot());
                let root_baseline_load = carried_load_of_actor(&root_baseline_state, actor);

                contract.is_satisfied(root_baseline_load)
            }
            GoalKind::ProduceCommodity { .. }
            | GoalKind::SearchForMissing { .. }
            | GoalKind::ReportMissing { .. }
            | GoalKind::ReportFound { .. }
            | GoalKind::ShareBelief { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::PostBounty { .. }
            | GoalKind::PostNotice { .. }
            | GoalKind::InvestigateViolation { .. }
            | GoalKind::Patrol { .. }
            | GoalKind::Accuse { .. }
            | GoalKind::PunishAccused { .. } => false,
        }
    }

    fn goal_relevant_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
    ) -> Vec<EntityId> {
        let actor = state.snapshot().actor();
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity } => {
                if state.commodity_quantity(actor, *commodity) > Quantity(0) {
                    state.effective_place(actor).into_iter().collect()
                } else {
                    places_with_resource_source(state, *commodity)
                }
            }
            GoalKind::AcquireCommodity { commodity, .. } => {
                let mut places = places_with_resource_source(state, *commodity);
                places_with_sellers(state, *commodity, &mut places);
                places
            }
            GoalKind::Relieve => {
                let mut places = places_with_place_tag(state, PlaceTag::Latrine);
                // Include outdoor places where wilderness relief is available.
                for (&id, place) in &state.snapshot().places {
                    if place.tags.iter().any(|t| OUTDOOR_RELIEF_TAGS.contains(*t)) {
                        places.push(id);
                    }
                }
                places.sort_unstable();
                places.dedup();
                places
            }
            GoalKind::EngageHostile { target }
            | GoalKind::RaidTarget { target }
            | GoalKind::TreatWounds { patient: target } => {
                state.effective_place(*target).into_iter().collect()
            }
            GoalKind::SearchForMissing { last_seen, .. } => last_seen.iter().copied().collect(),
            GoalKind::ReportMissing { to_office, .. } => match to_office {
                Some(office) => vec![*office],
                None => state.effective_place(actor).into_iter().collect(),
            },
            GoalKind::EscortToSafety { destination, .. } => vec![*destination],
            GoalKind::Wash => places_with_wash_access(state),
            GoalKind::ReportFound { .. }
            | GoalKind::Sleep
            | GoalKind::ReduceDanger
            | GoalKind::SupportCandidateForOffice { .. } => Vec::new(),
            GoalKind::FreeCarryCapacity => state.effective_place(actor).into_iter().collect(),
            GoalKind::RegroupWithFaction { faction } => {
                match state.believed_faction_rally_point(*faction) {
                    InstitutionalBeliefRead::Certain(Some(rally_place)) => vec![rally_place],
                    _ => Vec::new(),
                }
            }
            GoalKind::EstablishBanditCamp { faction } => {
                match state.believed_faction_rally_point(*faction) {
                    InstitutionalBeliefRead::Certain(Some(rally_place)) => vec![rally_place],
                    _ => state.effective_place(actor).into_iter().collect(),
                }
            }
            GoalKind::ClaimOffice { office } => {
                if office_succession_law(state, *office) == Some(SuccessionLaw::Force) {
                    state.snapshot().seat(*office).into_iter().collect()
                } else {
                    Vec::new()
                }
            }
            GoalKind::ProduceCommodity { recipe_id } => {
                let required_tag = recipes
                    .get(*recipe_id)
                    .and_then(|recipe| recipe.required_workstation_tag);
                match required_tag {
                    Some(tag) => places_with_workstation(state, tag),
                    None => Vec::new(),
                }
            }
            GoalKind::SellCommodity { .. } => state
                .merchandise_profile(actor)
                .and_then(|p| p.home_facility)
                .and_then(|facility| state.effective_place(facility))
                .into_iter()
                .collect(),
            GoalKind::RestockCommodity { commodity } => {
                if state.commodity_quantity(actor, *commodity) > Quantity(0) {
                    demand_memory_places(state, actor, *commodity)
                } else {
                    places_with_resource_source(state, *commodity)
                }
            }
            GoalKind::MoveCargo { destination, .. } => state
                .effective_place(*destination)
                .or(Some(*destination))
                .into_iter()
                .collect(),
            GoalKind::FulfillBounty { bounty } => {
                let Some(terms) = believed_bounty_terms(state, *bounty) else {
                    return Vec::new();
                };
                match terms.target {
                    BountyTarget::EliminateEntity { target }
                        if !state.is_dead(target) && state.effective_place(target).is_some() =>
                    {
                        state.effective_place(target).into_iter().collect()
                    }
                    BountyTarget::DeliverCommodity {
                        commodity,
                        quantity,
                        destination,
                    } if delivery_bounty_gap_at_destination(
                        state,
                        actor,
                        destination,
                        commodity,
                        quantity,
                    )
                    .is_some() =>
                    {
                        vec![destination]
                    }
                    _ => vec![terms.claim_place],
                }
            }
            GoalKind::PostBounty { posting, .. } | GoalKind::PostNotice { posting, .. } => {
                vec![posting.posting_place]
            }
            GoalKind::LootCorpse { corpse } | GoalKind::BuryCorpse { corpse, .. } => {
                state.effective_place(*corpse).into_iter().collect()
            }
            GoalKind::ShareBelief { listener, .. } => {
                state.effective_place(*listener).into_iter().collect()
            }
            GoalKind::AskWitness { witness, .. } => {
                state.effective_place(*witness).into_iter().collect()
            }
            GoalKind::InvestigateViolation { place, .. } | GoalKind::Patrol { place } => {
                vec![*place]
            }
            GoalKind::ExploreLocation { target_place, .. } => vec![*target_place],
            GoalKind::StealItem { target_item } => {
                state.effective_place(*target_item).into_iter().collect()
            }
            GoalKind::Accuse { crime_register, .. } => state
                .record_data(*crime_register)
                .map(|record| vec![record.home_place])
                .unwrap_or_default(),
            GoalKind::PunishAccused { accused, .. } => {
                state.effective_place(*accused).into_iter().collect()
            }
        }
    }

    fn prerequisite_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
        execution_budget: &ExecutionBudget,
    ) -> Vec<EntityId> {
        let actor = state.snapshot().actor();
        match self {
            GoalKind::TreatWounds { .. } => {
                if state.commodity_quantity(actor, CommodityKind::Medicine) > Quantity(0) {
                    Vec::new()
                } else {
                    acquisition_places_for_commodity(
                        state,
                        actor,
                        CommodityKind::Medicine,
                        execution_budget.max_prerequisite_locations(),
                    )
                }
            }
            GoalKind::ProduceCommodity { recipe_id } => {
                let Some(recipe) = recipes.get(*recipe_id) else {
                    return Vec::new();
                };
                prerequisite_places_for_recipe_inputs(
                    state,
                    actor,
                    std::iter::once(recipe),
                    execution_budget.max_prerequisite_locations(),
                )
            }
            GoalKind::RestockCommodity { commodity } => prerequisite_places_for_recipe_inputs(
                state,
                actor,
                recipes
                    .iter()
                    .filter(|(_, recipe)| {
                        recipe
                            .outputs
                            .iter()
                            .any(|(output, _)| *output == *commodity)
                    })
                    .map(|(_, recipe)| recipe),
                execution_budget.max_prerequisite_locations(),
            ),
            GoalKind::ClaimOffice { office }
            | GoalKind::SupportCandidateForOffice { office, .. } => {
                if matches!(self, GoalKind::ClaimOffice { .. })
                    && office_succession_law(state, *office) == Some(SuccessionLaw::Force)
                {
                    return state.snapshot().seat(*office).into_iter().collect();
                }
                if state.believed_office_holder(*office) != InstitutionalBeliefRead::Unknown {
                    return Vec::new();
                }
                office_register_for_goal(state, *office)
                    .map(|(_, home_place, _)| vec![home_place])
                    .unwrap_or_default()
            }
            GoalKind::SellCommodity { .. } => state
                .merchandise_profile(actor)
                .and_then(|p| p.home_facility)
                .and_then(|facility| state.effective_place(facility))
                .filter(|home_place| state.effective_place(actor) != Some(*home_place))
                .into_iter()
                .collect(),
            GoalKind::AskWitness { witness, .. } => {
                state.effective_place(*witness).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    fn matches_binding(&self, authoritative_targets: &[EntityId], op_kind: PlannerOpKind) -> bool {
        // Planner-only synthetic candidates have empty targets — always pass.
        if authoritative_targets.is_empty() {
            return true;
        }

        // Auxiliary ops serve the goal indirectly — always pass.
        match op_kind {
            PlannerOpKind::Travel
            | PlannerOpKind::Trade
            | PlannerOpKind::StaffMarket
            | PlannerOpKind::StockManagement
            | PlannerOpKind::EstablishCamp
            | PlannerOpKind::Harvest
            | PlannerOpKind::Craft
            | PlannerOpKind::QueueForFacilityUse
            | PlannerOpKind::MoveCargo
            | PlannerOpKind::DropItem
            | PlannerOpKind::Consume
            | PlannerOpKind::Sleep
            | PlannerOpKind::Relieve
            | PlannerOpKind::Wash
            | PlannerOpKind::Patrol
            | PlannerOpKind::Defend
            | PlannerOpKind::Bribe
            | PlannerOpKind::Threaten
            | PlannerOpKind::ConsultRecord => return true,
            // Terminal ops — fall through to goal-specific binding check.
            PlannerOpKind::Attack
            | PlannerOpKind::Loot
            | PlannerOpKind::Heal
            | PlannerOpKind::Tell
            | PlannerOpKind::DeclareSupport
            | PlannerOpKind::PressForceClaim
            | PlannerOpKind::YieldForceClaim
            | PlannerOpKind::Accuse
            | PlannerOpKind::Fine
            | PlannerOpKind::Exile
            | PlannerOpKind::Investigate
            | PlannerOpKind::AskWitness
            | PlannerOpKind::ClaimBounty
            | PlannerOpKind::WithdrawBounty
            | PlannerOpKind::PostBounty
            | PlannerOpKind::PostNotice
            | PlannerOpKind::Bury
            | PlannerOpKind::SearchPlace
            | PlannerOpKind::AskAboutPerson
            | PlannerOpKind::ReportMissing
            | PlannerOpKind::EscortToSafety
            | PlannerOpKind::ReportFound => {}
        }

        // Terminal ops on flexible goals — always pass.
        // Terminal ops on exact-bound goals — verify target identity.
        match self {
            // Flexible goals and DeclareSupport edge case: no binding requirement.
            // ClaimOffice/SupportCandidateForOffice have empty bound_targets in
            // practice (handled by the empty-targets bypass above). If non-empty,
            // payload override handles correctness.
            GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::AcquireCommodity { .. }
            | GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::FreeCarryCapacity
            | GoalKind::ReduceDanger
            | GoalKind::RegroupWithFaction { .. }
            | GoalKind::EstablishBanditCamp { .. }
            | GoalKind::SearchForMissing { .. }
            | GoalKind::ReportMissing { .. }
            | GoalKind::ReportFound { .. }
            | GoalKind::ProduceCommodity { .. }
            | GoalKind::SellCommodity { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::ClaimOffice { .. }
            | GoalKind::SupportCandidateForOffice { .. } => true,

            GoalKind::ExploreLocation { target_place, .. } => {
                authoritative_targets.contains(target_place)
            }

            GoalKind::InvestigateViolation { place, .. } | GoalKind::Patrol { place } => {
                authoritative_targets.contains(place)
            }

            // Exact-bound goals: target must match.
            GoalKind::EngageHostile { target }
            | GoalKind::RaidTarget { target }
            | GoalKind::TreatWounds { patient: target }
            | GoalKind::StealItem {
                target_item: target,
            }
            | GoalKind::Accuse {
                accused: target, ..
            }
            | GoalKind::PunishAccused {
                accused: target, ..
            } => authoritative_targets.contains(target),
            GoalKind::LootCorpse { corpse } => authoritative_targets.contains(corpse),
            GoalKind::FulfillBounty { bounty } => {
                op_kind != PlannerOpKind::ClaimBounty || authoritative_targets.contains(bounty)
            }
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            } => {
                authoritative_targets.contains(corpse)
                    || authoritative_targets.contains(burial_site)
            }
            GoalKind::ShareBelief { listener, .. } => authoritative_targets.contains(listener),
            GoalKind::AskWitness { witness, .. } => authoritative_targets.contains(witness),
            GoalKind::MoveCargo { destination, .. } => authoritative_targets.contains(destination),
            GoalKind::EscortToSafety {
                subject,
                destination,
            } => {
                authoritative_targets.contains(subject)
                    || authoritative_targets.contains(destination)
            }
            GoalKind::PostBounty { posting, .. } | GoalKind::PostNotice { posting, .. } => {
                authoritative_targets.contains(&posting.posting_place)
            }
        }
    }

    fn candidate_is_available(&self, state: &PlanningState<'_>, op_kind: PlannerOpKind) -> bool {
        match (self, op_kind) {
            (GoalKind::FreeCarryCapacity, PlannerOpKind::DropItem) => {
                !free_carry_capacity_drop_targets(state).is_empty()
            }
            (GoalKind::FreeCarryCapacity, _) => false,
            (GoalKind::FulfillBounty { bounty }, op_kind) => {
                let Some(terms) = believed_bounty_terms(state, *bounty) else {
                    return false;
                };
                let actor_at_claim_place =
                    state.effective_place(state.snapshot().actor()) == Some(terms.claim_place);
                match terms.target {
                    BountyTarget::EliminateEntity { target } => match op_kind {
                        PlannerOpKind::Attack => !state.is_dead(target),
                        PlannerOpKind::ClaimBounty => state.is_dead(target) && actor_at_claim_place,
                        PlannerOpKind::MoveCargo | PlannerOpKind::StockManagement => false,
                        _ => true,
                    },
                    BountyTarget::DeliverCommodity {
                        commodity,
                        quantity,
                        destination,
                    } => match op_kind {
                        PlannerOpKind::Attack => false,
                        PlannerOpKind::ClaimBounty => {
                            delivery_bounty_gap_at_destination(
                                state,
                                state.snapshot().actor(),
                                destination,
                                commodity,
                                quantity,
                            )
                            .is_none()
                                && actor_at_claim_place
                        }
                        _ => true,
                    },
                }
            }
            (GoalKind::Accuse { crime_register, .. }, PlannerOpKind::Accuse) => {
                let Some(record) = state.record_data(*crime_register) else {
                    return false;
                };
                state.effective_place(state.snapshot().actor()) == Some(record.home_place)
            }
            (GoalKind::SearchForMissing { last_seen, .. }, PlannerOpKind::AskAboutPerson) => state
                .effective_place(state.snapshot().actor())
                .is_none_or(|actor_place| Some(actor_place) != *last_seen),
            (GoalKind::AskWitness { witness, topic }, PlannerOpKind::AskWitness) => {
                ask_witness_candidate_available(state, *witness, *topic)
            }
            _ => true,
        }
    }
}

fn ask_witness_goal_satisfied(
    state: &PlanningState<'_>,
    actor: EntityId,
    witness: EntityId,
    topic: TellTopic,
) -> bool {
    let Some(subject) = topic_entity_subject(topic) else {
        return false;
    };
    let Some(profile) = state.epistemic_disposition_profile(actor) else {
        return false;
    };
    let confidence_policy = state.belief_confidence_policy(actor);
    let current_tick = state.current_tick();
    state
        .entity_beliefs_sourced_from_witness(actor, witness)
        .into_iter()
        .any(|(entity, belief)| {
            if entity != subject
                || !matches!(belief.source, PerceptionSource::Report { from, .. } if from == witness)
            {
                return false;
            }
            let staleness_ticks = current_tick
                .0
                .saturating_sub(belief.last_observed_tick().unwrap_or(Tick(0)).0);
            let confidence_satisfies =
                belief_confidence(&belief.source, staleness_ticks, &confidence_policy)
                    >= profile.stale_evidence_barrier_threshold;
            let freshness_satisfies = report_is_fresh_enough_for_witness_preference(
                staleness_ticks,
                &profile,
                &confidence_policy,
            );
            confidence_satisfies || freshness_satisfies
        })
}

fn ask_witness_candidate_available(
    state: &PlanningState<'_>,
    witness: EntityId,
    topic: TellTopic,
) -> bool {
    let actor = state.snapshot().actor();
    let Some(subject) = topic_entity_subject(topic) else {
        return false;
    };
    let Some(profile) = state.epistemic_disposition_profile(actor) else {
        return false;
    };
    let confidence_policy = state.belief_confidence_policy(actor);
    let Some(belief) = state
        .known_entity_beliefs(actor)
        .into_iter()
        .find_map(|(entity, belief)| (entity == subject).then_some(belief))
    else {
        return false;
    };
    let staleness_ticks = state
        .current_tick()
        .0
        .saturating_sub(belief.last_observed_tick().unwrap_or(Tick(0)).0);
    if belief_confidence(&belief.source, staleness_ticks, &confidence_policy)
        >= profile.stale_evidence_barrier_threshold
    {
        return false;
    }
    let key = worldwake_core::AskWitnessMemoryKey {
        counterparty: witness,
        topic_entity: Some(subject),
        topic_commodity: None,
    };
    state.ask_witness_memory(actor, &key).is_none_or(|memory| {
        state.current_tick().0.saturating_sub(memory.asked_tick.0)
            >= u64::from(profile.ask_memory_retention_ticks)
    })
}

fn carried_load_of_actor(state: &PlanningState<'_>, actor: EntityId) -> Option<LoadUnits> {
    let actor_ref = PlanningEntityRef::Authoritative(actor);
    let capacity = state.carry_capacity_ref(actor_ref)?.0;
    let remaining = state.remaining_carry_capacity_ref(actor_ref)?.0;
    capacity.checked_sub(remaining).map(LoadUnits)
}

fn free_carry_capacity_drop_targets(state: &PlanningState<'_>) -> Vec<PlanningEntityRef> {
    state
        .direct_possessions_ref(PlanningEntityRef::Authoritative(state.snapshot().actor()))
        .into_iter()
        .filter(|entity| {
            state.entity_kind_ref(*entity) == Some(worldwake_core::EntityKind::ItemLot)
        })
        .filter(|entity| state.item_lot_commodity_ref(*entity) == Some(CommodityKind::Waste))
        .filter(|entity| {
            state
                .item_lot_commodity_ref(*entity)
                .is_some_and(|commodity| {
                    state.commodity_quantity_ref(*entity, commodity) > Quantity(0)
                })
        })
        .collect()
}

/// Collect places containing entities with a `ResourceSource` for the given commodity.
fn places_with_resource_source(
    state: &PlanningState<'_>,
    commodity: CommodityKind,
) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state
            .resource_source(entity_id)
            .is_some_and(|s| s.commodity == commodity && s.available_quantity > Quantity(0))
            && let Some(place) = state.effective_place(entity_id)
        {
            places.insert(place);
        }
    }
    places.into_iter().collect()
}

fn places_with_wash_access(state: &PlanningState<'_>) -> Vec<EntityId> {
    places_with_workstation(state, WorkstationTag::WashBasin)
        .into_iter()
        .filter(|place| {
            state
                .matching_workstations_at(*place, WorkstationTag::WashBasin)
                .iter()
                .any(|basin| {
                    // FND-14A: the planner stages wash plans only when the
                    // agent has observed the basin's state — directly via
                    // co-location or via `BelievedEntityState::wash_basin_state`
                    // surfaced by `FacilityBeliefView::wash_basin_state`.
                    FacilityBeliefView::wash_basin_state(state, *basin)
                        .is_some_and(|basin_state| basin_state.clean_water_units > 0)
                })
        })
        .collect()
}

fn delivery_bounty_gap_at_destination(
    state: &PlanningState<'_>,
    actor: EntityId,
    destination: EntityId,
    commodity: CommodityKind,
    required_quantity: Quantity,
) -> Option<Quantity> {
    let delivered = state.controlled_commodity_quantity_at_place(actor, destination, commodity);
    (delivered < required_quantity).then_some(Quantity(required_quantity.0 - delivered.0))
}

/// Append places where merchants are selling the given commodity (deduplicating with `existing`).
fn places_with_sellers(
    state: &PlanningState<'_>,
    commodity: CommodityKind,
    existing: &mut Vec<EntityId>,
) {
    let already: BTreeSet<EntityId> = existing.iter().copied().collect();
    for &entity_id in state.snapshot().entities.keys() {
        if let Some(profile) = state.merchandise_profile(entity_id)
            && profile.sale_kinds.contains(&commodity)
            && let Some(place) = state.effective_place(entity_id)
            && !already.contains(&place)
            && !existing.contains(&place)
        {
            existing.push(place);
        }
    }
}

fn places_with_seller_list(state: &PlanningState<'_>, commodity: CommodityKind) -> Vec<EntityId> {
    let mut places = Vec::new();
    places_with_sellers(state, commodity, &mut places);
    places
}

fn places_with_loose_lots(state: &PlanningState<'_>, commodity: CommodityKind) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state.item_lot_commodity(entity_id) != Some(commodity) {
            continue;
        }
        if state.commodity_quantity(entity_id, commodity) == Quantity(0) {
            continue;
        }
        if state.direct_possessor(entity_id).is_some()
            || state.direct_container(entity_id).is_some()
        {
            continue;
        }
        if let Some(place) = state.effective_place(entity_id) {
            places.insert(place);
        }
    }
    places.into_iter().collect()
}

fn acquisition_places_for_commodity(
    state: &PlanningState<'_>,
    actor: EntityId,
    commodity: CommodityKind,
    limit: u8,
) -> Vec<EntityId> {
    let loose_lot_places = places_with_loose_lots(state, commodity);
    if !loose_lot_places.is_empty() {
        let result = cap_places_by_travel_distance(state, actor, loose_lot_places, limit);
        return result;
    }
    let mut places = places_with_seller_list(state, commodity);
    append_unique_places(&mut places, places_with_resource_source(state, commodity));
    cap_places_by_travel_distance(state, actor, places, limit)
}

fn prerequisite_places_for_recipe_inputs<'a>(
    state: &PlanningState<'_>,
    actor: EntityId,
    recipes: impl Iterator<Item = &'a RecipeDefinition>,
    limit: u8,
) -> Vec<EntityId> {
    let mut places = Vec::new();
    for recipe in recipes {
        for (commodity, required_quantity) in &recipe.inputs {
            if state.commodity_quantity(actor, *commodity) >= *required_quantity {
                continue;
            }
            append_unique_places(
                &mut places,
                acquisition_places_for_commodity(state, actor, *commodity, limit),
            );
        }
    }
    cap_places_by_travel_distance(state, actor, places, limit)
}

pub(crate) fn trace_prerequisite_guidance(
    goal_relevant_places: Vec<EntityId>,
    prerequisite_places: Vec<EntityId>,
    exclusions: Vec<PrerequisiteExclusionTrace>,
) -> Option<PrerequisiteGuidanceTrace> {
    (!goal_relevant_places.is_empty() || !prerequisite_places.is_empty() || !exclusions.is_empty())
        .then_some(PrerequisiteGuidanceTrace {
            goal_relevant_places,
            prerequisite_places,
            exclusions,
        })
}

pub(crate) fn prerequisite_depleted_source_exclusions(
    goal: &GoalKind,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> Vec<PrerequisiteExclusionTrace> {
    let actor = state.snapshot().actor();
    let mut exclusions = BTreeSet::new();
    match goal {
        GoalKind::TreatWounds { .. } => {
            exclusions.extend(depleted_source_exclusions_for_acquisition(
                state,
                CommodityKind::Medicine,
            ));
        }
        GoalKind::ProduceCommodity { recipe_id } => {
            let Some(recipe) = recipes.get(*recipe_id) else {
                return Vec::new();
            };
            exclusions.extend(missing_input_depleted_source_exclusions(
                state,
                actor,
                std::iter::once(recipe),
            ));
        }
        GoalKind::RestockCommodity { commodity } => {
            exclusions.extend(missing_input_depleted_source_exclusions(
                state,
                actor,
                recipes
                    .iter()
                    .filter(|(_, recipe)| {
                        recipe
                            .outputs
                            .iter()
                            .any(|(output, _)| *output == *commodity)
                    })
                    .map(|(_, recipe)| recipe),
            ));
        }
        _ => {}
    }
    exclusions.into_iter().collect()
}

fn missing_input_depleted_source_exclusions<'a>(
    state: &PlanningState<'_>,
    actor: EntityId,
    recipes: impl Iterator<Item = &'a RecipeDefinition>,
) -> BTreeSet<PrerequisiteExclusionTrace> {
    let mut exclusions = BTreeSet::new();
    for recipe in recipes {
        for (commodity, required_quantity) in &recipe.inputs {
            if state.commodity_quantity(actor, *commodity) >= *required_quantity {
                continue;
            }
            exclusions.extend(depleted_source_exclusions_for_acquisition(
                state, *commodity,
            ));
        }
    }
    exclusions
}

fn depleted_source_exclusions_for_acquisition(
    state: &PlanningState<'_>,
    commodity: CommodityKind,
) -> BTreeSet<PrerequisiteExclusionTrace> {
    if !places_with_loose_lots(state, commodity).is_empty() {
        return BTreeSet::new();
    }

    let mut exclusions = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state.resource_source(entity_id).is_some_and(|source| {
            source.commodity == commodity && source.available_quantity == Quantity(0)
        }) && let Some(place) = state.effective_place(entity_id)
        {
            exclusions.insert(PrerequisiteExclusionTrace {
                place,
                commodity,
                reason: PrerequisiteExclusionReason::DepletedResourceSource,
            });
        }
    }
    exclusions
}

fn append_unique_places(existing: &mut Vec<EntityId>, new_places: Vec<EntityId>) {
    for place in new_places {
        if !existing.contains(&place) {
            existing.push(place);
        }
    }
}

fn cap_places_by_travel_distance(
    state: &PlanningState<'_>,
    actor: EntityId,
    mut places: Vec<EntityId>,
    limit: u8,
) -> Vec<EntityId> {
    if limit == 0 {
        return Vec::new();
    }
    if usize::from(limit) >= places.len() {
        return places;
    }
    let actor_place = state.effective_place(actor);
    places.sort_by_key(|place| {
        (
            actor_place
                .and_then(|from| state.snapshot().min_travel_ticks(from, *place))
                .unwrap_or(u32::MAX),
            *place,
        )
    });
    places.truncate(usize::from(limit));
    places
}

/// Collect places with the given `PlaceTag`.
fn places_with_place_tag(state: &PlanningState<'_>, tag: PlaceTag) -> Vec<EntityId> {
    state
        .snapshot()
        .places
        .iter()
        .filter(|(_, place)| place.tags.contains(&tag))
        .map(|(id, _)| *id)
        .collect()
}

/// Collect places containing entities with the given `WorkstationTag`.
fn places_with_workstation(state: &PlanningState<'_>, tag: WorkstationTag) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state.workstation_tag(entity_id) == Some(tag)
            && let Some(place) = state.effective_place(entity_id)
        {
            places.insert(place);
        }
    }
    places.into_iter().collect()
}

/// Collect places from the actor's demand memory for the given commodity,
/// filtered to places present in the planning snapshot.
fn demand_memory_places(
    state: &PlanningState<'_>,
    actor: EntityId,
    commodity: CommodityKind,
) -> Vec<EntityId> {
    let snapshot_places = &state.snapshot().places;
    let places: BTreeSet<EntityId> = state
        .demand_memory(actor)
        .into_iter()
        .filter(|obs| obs.commodity == commodity)
        .map(|obs| obs.place)
        .filter(|place| snapshot_places.contains_key(place))
        .collect();
    places.into_iter().collect()
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalPriorityClass {
    Background,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RankedDriveKind {
    Hunger,
    Thirst,
    Fatigue,
    Bladder,
    Dirtiness,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RankedPriorityAdjustment {
    ClottedWoundRecoveryPromotion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedDriveMotiveInput {
    pub drive: RankedDriveKind,
    pub pressure: Permille,
    pub weight: Permille,
    pub score: u32,
    pub escalation_multiplier: MultiplierPermille,
    pub relief_per_unit: Permille,
    pub recovery_relevant: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedDriveGoalProvenance {
    pub base_priority_class: GoalPriorityClass,
    pub final_priority_class: GoalPriorityClass,
    pub adjustment: Option<RankedPriorityAdjustment>,
    pub commodity_preference_rank: Option<u8>,
    pub motive_inputs: Vec<RankedDriveMotiveInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RankedGoalProvenance {
    Danger(DangerAssessment),
    Drive(RankedDriveGoalProvenance),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalOffer {
    pub key: GoalKey,
    pub anchor: worldwake_core::OpportunityAnchor,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
    pub obligation_source: Option<EntityId>,
    pub commitment_impact_if_ignored: Permille,
    pub required_information_gaps: Vec<worldwake_core::BeliefClaimKey>,
    pub invalidators: Vec<crate::Invalidator>,
    pub learned_expectation_refs: Vec<worldwake_core::ExpectationId>,
    pub motive_sources: Vec<worldwake_core::MotiveSourceRef>,
    /// Per-emission `AcquisitionQuantity` preserved alongside the normalized
    /// `GoalKey`. `Some` when the offer's `kind` was `AcquireCommodity`;
    /// `None` for all other goal families. Surfaces the per-agent
    /// `desired_min` / `desired_target` / `horizon_ticks` through the
    /// decision-trace pipeline (FND-29) without affecting goal identity
    /// (S127 Design Goal 9).
    pub acquisition_quantity: Option<AcquisitionQuantity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RootCandidateSynthesis {
    Targets(Vec<EntityId>),
    NoSynthesisPath,
    UnsupportedGoalOp,
    TargetDerivationFailed,
}

impl GoalOffer {
    #[track_caller]
    pub fn assert_motive_sources_present(&self) {
        debug_assert!(
            !self.motive_sources.is_empty(),
            "GoalOffer.motive_sources must be non-empty post-S141"
        );
    }

    fn can_synthesize_actor_place_root(actor_place: Option<EntityId>, place: EntityId) -> bool {
        actor_place == Some(place)
    }

    fn can_synthesize_entity_at_actor_place_root(&self, actor_place: Option<EntityId>) -> bool {
        actor_place.is_some_and(|place| self.evidence_places.contains(&place))
    }

    pub(crate) fn synthesized_root_candidate_targets(
        &self,
        def: &ActionDef,
        semantics: PlannerOpSemantics,
        actor_place: Option<EntityId>,
    ) -> RootCandidateSynthesis {
        match semantics.op_kind {
            PlannerOpKind::Trade => match &self.key.kind {
                GoalKind::AcquireCommodity { .. }
                | GoalKind::ConsumeOwnedCommodity { .. }
                | GoalKind::RestockCommodity { .. }
                | GoalKind::TreatWounds { .. } => {
                    if !matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    if self.evidence_entities.len() != 1 {
                        return RootCandidateSynthesis::TargetDerivationFailed;
                    }
                    RootCandidateSynthesis::Targets(
                        self.evidence_entities.iter().copied().collect(),
                    )
                }
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Harvest => match &self.key.kind {
                GoalKind::AcquireCommodity { .. } | GoalKind::RestockCommodity { .. } => {
                    if !matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    if self.evidence_entities.len() != 1 {
                        return RootCandidateSynthesis::TargetDerivationFailed;
                    }
                    RootCandidateSynthesis::Targets(
                        self.evidence_entities.iter().copied().collect(),
                    )
                }
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Wash => match &self.key.kind {
                GoalKind::Wash
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) =>
                {
                    if actor_place.is_none_or(|place| !self.evidence_places.contains(&place)) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    if let worldwake_core::OpportunityAnchor::Entity(basin) = self.anchor {
                        RootCandidateSynthesis::Targets(vec![basin])
                    } else if self.evidence_entities.len() == 1 {
                        RootCandidateSynthesis::Targets(
                            self.evidence_entities.iter().copied().collect(),
                        )
                    } else {
                        RootCandidateSynthesis::TargetDerivationFailed
                    }
                }
                GoalKind::Wash => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::PressForceClaim => match &self.key.kind {
                GoalKind::ClaimOffice { .. } if def.targets.is_empty() => {
                    RootCandidateSynthesis::Targets(Vec::new())
                }
                GoalKind::ClaimOffice { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Investigate => match &self.key.kind {
                // Investigate is lawful as a direct root only when the actor is
                // already co-located with the violation place. Remote investigate
                // must flow through travel prerequisites instead of an impossible
                // direct ActorPlace root.
                GoalKind::InvestigateViolation { place, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::ActorPlace]
                    ) && actor_place == Some(*place) =>
                {
                    RootCandidateSynthesis::Targets(vec![*place])
                }
                GoalKind::InvestigateViolation { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::SearchPlace => match &self.key.kind {
                GoalKind::SearchForMissing { .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::ActorPlace]
                    ) && actor_place.is_some() =>
                {
                    RootCandidateSynthesis::Targets(vec![actor_place.expect("checked is_some")])
                }
                GoalKind::SearchForMissing { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::ReportMissing => match &self.key.kind {
                GoalKind::ReportMissing {
                    to_office: None, ..
                } if matches!(
                    def.targets.as_slice(),
                    [worldwake_sim::TargetSpec::ActorPlace]
                ) && actor_place.is_some() =>
                {
                    RootCandidateSynthesis::Targets(vec![actor_place.expect("checked is_some")])
                }
                GoalKind::ReportMissing { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::EstablishCamp => match &self.key.kind {
                GoalKind::EstablishBanditCamp { .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::ActorPlace]
                    ) =>
                {
                    let Some(place) = (match self.anchor {
                        worldwake_core::OpportunityAnchor::Place(place) => Some(place),
                        _ => self.key.place,
                    }) else {
                        return RootCandidateSynthesis::TargetDerivationFailed;
                    };
                    if !Self::can_synthesize_actor_place_root(actor_place, place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    RootCandidateSynthesis::Targets(vec![place])
                }
                GoalKind::EstablishBanditCamp { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Attack => match &self.key.kind {
                GoalKind::EngageHostile { .. } | GoalKind::RaidTarget { .. } => {
                    RootCandidateSynthesis::NoSynthesisPath
                }
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Tell => match &self.key.kind {
                GoalKind::ShareBelief { listener, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) =>
                {
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    RootCandidateSynthesis::Targets(vec![*listener])
                }
                GoalKind::ShareBelief { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::AskWitness => match &self.key.kind {
                GoalKind::AskWitness { witness, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) =>
                {
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    RootCandidateSynthesis::Targets(vec![*witness])
                }
                GoalKind::AskWitness { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Accuse => match &self.key.kind {
                GoalKind::Accuse { accused, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::SpecificEntity(_)]
                    ) =>
                {
                    RootCandidateSynthesis::Targets(vec![*accused])
                }
                GoalKind::Accuse { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::Fine | PlannerOpKind::Exile => match &self.key.kind {
                GoalKind::PunishAccused { accused, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) =>
                {
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    RootCandidateSynthesis::Targets(vec![*accused])
                }
                GoalKind::PunishAccused { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::ClaimBounty => match &self.key.kind {
                GoalKind::FulfillBounty { bounty }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::SpecificEntity(_)]
                    ) =>
                {
                    RootCandidateSynthesis::Targets(vec![*bounty])
                }
                GoalKind::FulfillBounty { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::PostBounty => match &self.key.kind {
                GoalKind::PostBounty { posting, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::ActorPlace]
                    ) && actor_place == Some(posting.posting_place) =>
                {
                    RootCandidateSynthesis::Targets(vec![posting.posting_place])
                }
                GoalKind::PostBounty { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::PostNotice => match &self.key.kind {
                GoalKind::PostNotice { posting, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::ActorPlace]
                    ) && actor_place == Some(posting.posting_place) =>
                {
                    RootCandidateSynthesis::Targets(vec![posting.posting_place])
                }
                GoalKind::PostNotice { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            PlannerOpKind::EscortToSafety => match &self.key.kind {
                GoalKind::EscortToSafety { subject, .. }
                    if matches!(
                        def.targets.as_slice(),
                        [worldwake_sim::TargetSpec::EntityAtActorPlace { .. }]
                    ) =>
                {
                    if !self.can_synthesize_entity_at_actor_place_root(actor_place) {
                        return RootCandidateSynthesis::NoSynthesisPath;
                    }
                    RootCandidateSynthesis::Targets(vec![*subject])
                }
                GoalKind::EscortToSafety { .. } => RootCandidateSynthesis::NoSynthesisPath,
                _ => RootCandidateSynthesis::UnsupportedGoalOp,
            },
            _ => RootCandidateSynthesis::UnsupportedGoalOp,
        }
    }
}

pub use crate::agenda_types::AgendaEntry;

#[cfg(test)]
mod tests {
    use super::{
        AgendaEntry, GoalKindPlannerExt, GoalOffer, GoalPayloadOverrideError, GoalPriorityClass,
        RankedGoalProvenanceFamily, RootCandidateSynthesis, grounded_goal_epistemic_subjects,
        grounded_goal_matches_epistemic_barrier,
    };
    use crate::{
        CommodityPurpose, GoalKey, GoalKind, PlannedStep, PlannerOpKind, PlannerOpSemantics,
        PlannerSyntheticCargo, PlanningState, ProfileFixture, build_planning_snapshot,
        build_semantics_table,
        decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;
    use std::num::NonZeroU32;
    use worldwake_core::ActionDomain;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, AgentBeliefStore, ArtifactActionability,
        ArtifactCredibility, ArtifactExistence, ArtifactKind, ArtifactLegalEffect,
        ArtifactPostingContext, ArtifactVisibility, AskWitnessMemory, AskWitnessMemoryKey,
        BelievedArtifactState, BelievedBountyTerms, BelievedEntityState,
        BelievedInstitutionalClaim, BlockerMemory, BodyCostPerTick, BountyTarget, BountyTerms,
        CloseCause, CognitiveProfile, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DemandObservationReason, DeprivationExposure, DisposalProfile,
        DriveEscalationProfile, DriveThresholds, EntityId, EntityKind, EpistemicDispositionProfile,
        EpistemicSubject, ExecutionBudget, HomeostaticNeedId, HomeostaticNeeds, InTransitOnEdge,
        InstitutionalBeliefRead, InstitutionalClaim, InstitutionalKnowledgeSource, LoadUnits,
        MerchandiseProfile, MetabolismProfile, NoticeTopic, OfficeData, PerceptionSource, Permille,
        ProofRequirement, PunishmentKind, Quantity, RecipeId, RecordEntryId, RecordKind,
        ResourceSource, RewardSource, SocialObservation, SocialObservationDetail, SuccessionLaw,
        TellTopic, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, ViolationId,
        VisibilitySpec, WashBasinState, WorkstationTag, Wound,
        test_utils::{entity_id, sample_trade_disposition_profile},
    };

    use worldwake_sim::{
        AccuseActionPayload, ActionDef, ActionDefRegistry, ActionDuration, ActionHandlerId,
        ActionPayload, AskAboutPersonActionPayload, AskWitnessPayload, ControlBeliefView,
        DurationExpr, EntityBeliefView, Interruptibility, InvestigateActionPayload,
        ProfileBeliefView, PunishActionPayload, QueueForFacilityUsePayload, RecipeRegistry,
        ReportMissingActionPayload, RuntimeBeliefView, SearchPlaceActionPayload, SpatialBeliefView,
        TellActionPayload, TemporalBeliefView, TradeActionPayload, TransportActionPayload,
        estimate_duration_from_beliefs,
    };
    use worldwake_systems::build_full_action_registries;

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn believed_bounty_artifact(
        issuer: EntityId,
        claim_place: EntityId,
        target: BountyTarget,
        actionability: ArtifactActionability,
        observed_tick: Tick,
    ) -> BelievedArtifactState {
        let legal_effect = match actionability {
            ArtifactActionability::Actionable => ArtifactLegalEffect::Active { expires_at: None },
            ArtifactActionability::Closed { closed_at, .. } => ArtifactLegalEffect::Fulfilled {
                fulfilled_at: closed_at,
                by: issuer,
                evidence: claim_place,
            },
            ArtifactActionability::AwaitingProof { .. } | ArtifactActionability::Blocked { .. } => {
                ArtifactLegalEffect::Active { expires_at: None }
            }
        };
        BelievedArtifactState {
            kind: ArtifactKind::Bounty,
            issuer,
            expires_at: None,
            existence: ArtifactExistence::Exists,
            visibility: ArtifactVisibility::Posted { place: claim_place },
            legal_effect,
            credibility: ArtifactCredibility::Credible,
            actionability,
            bounty_terms: Some(BelievedBountyTerms {
                target,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(20),
                claim_place,
            }),
            notice_topic: None,
            observed_tick,
        }
    }

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_per_expansion: CognitiveProfile::default().max_candidates_per_expansion,
            max_plan_depth: reasoning.max_plan_depth,
            max_travel_candidates_per_expansion: CognitiveProfile::default()
                .max_travel_candidates_per_expansion,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            planning_switch_margin: CognitiveProfile::default().planning_switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            stale_belief_backoff_ticks: CognitiveProfile::default().stale_belief_backoff_ticks,
            contradicted_belief_backoff_ticks: CognitiveProfile::default()
                .contradicted_belief_backoff_ticks,
            improper_state_backoff_ticks: CognitiveProfile::default().improper_state_backoff_ticks,
            missing_observation_backoff_ticks: CognitiveProfile::default()
                .missing_observation_backoff_ticks,
            no_legal_binding_backoff_ticks: CognitiveProfile::default()
                .no_legal_binding_backoff_ticks,
            counterparty_refusal_backoff_ticks: CognitiveProfile::default()
                .counterparty_refusal_backoff_ticks,
            route_unknown_backoff_ticks: CognitiveProfile::default().route_unknown_backoff_ticks,
            route_segment_blocker_ticks: CognitiveProfile::default().route_segment_blocker_ticks,
            counterparty_blocker_ticks: CognitiveProfile::default().counterparty_blocker_ticks,
            search_exhaustion_backoff_ticks: CognitiveProfile::default()
                .search_exhaustion_backoff_ticks,
            partial_drift_backoff_ticks: CognitiveProfile::default().partial_drift_backoff_ticks,
            expectation_tolerance_ticks: CognitiveProfile::default().expectation_tolerance_ticks,
            guard_min_confidence_ceiling: CognitiveProfile::default().guard_min_confidence_ceiling,
            repair_memory_ticks: CognitiveProfile::default().repair_memory_ticks,
            learned_opportunity_memory_ticks: CognitiveProfile::default()
                .learned_opportunity_memory_ticks,
            survey_memory_capacity: CognitiveProfile::default().survey_memory_capacity,
            survey_memory_retention_ticks: CognitiveProfile::default()
                .survey_memory_retention_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
            landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
            use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic,
            decision_history_alternatives: CognitiveProfile::default()
                .decision_history_alternatives,
            detour_budget_permille: CognitiveProfile::default().detour_budget_permille,
            compile_opportunity_cap: CognitiveProfile::default().compile_opportunity_cap,
            repair_budget_fraction: CognitiveProfile::default().repair_budget_fraction,
            causal_links_per_step_cap: CognitiveProfile::default().causal_links_per_step_cap,
        }
    }

    fn execution_budget(reasoning: &ProfileFixture) -> ExecutionBudget {
        ExecutionBudget::new(
            reasoning.beam_width,
            reasoning.max_prerequisite_locations,
            ExecutionBudget::default().preferred_operator_boost(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn search_plan(
        snapshot: &crate::PlanningSnapshot,
        goal: &GoalOffer,
        semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
        registry: &ActionDefRegistry,
        handlers: &worldwake_sim::ActionHandlerRegistry,
        reasoning: &ProfileFixture,
        recipes: &RecipeRegistry,
        blocked: &BlockerMemory,
        current_tick: Tick,
        binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
        expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    ) -> crate::PlanSearchResult {
        crate::search_plan(
            snapshot,
            goal,
            semantics_table,
            registry,
            handlers,
            &cognitive(reasoning),
            &execution_budget(reasoning),
            recipes,
            blocked,
            current_tick,
            binding_rejections,
            expansion_summaries,
        )
    }

    fn vacant_office(title: &str, jurisdiction: EntityId, faction: EntityId) -> OfficeData {
        OfficeData {
            title: title.to_string(),
            seat: jurisdiction,
            jurisdiction: BTreeSet::from([jurisdiction]),
            succession_law: SuccessionLaw::Support,
            eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(faction)],
            succession_period_ticks: 10,
            vacancy_since: Some(Tick(1)),
        }
    }

    #[test]
    fn goal_priority_class_satisfies_required_bounds() {
        assert_value_bounds::<GoalPriorityClass>();
        assert!(GoalPriorityClass::Critical > GoalPriorityClass::High);
        assert!(GoalPriorityClass::High > GoalPriorityClass::Medium);
        assert!(GoalPriorityClass::Medium > GoalPriorityClass::Low);
        assert!(GoalPriorityClass::Low > GoalPriorityClass::Background);
    }

    #[test]
    fn grounded_goal_satisfies_required_bounds() {
        assert_value_bounds::<GoalOffer>();
        assert_value_bounds::<AgendaEntry>();
    }

    #[test]
    fn ranked_goal_supports_optional_competition_discount() {
        let discount = CompetitionDiscount {
            observed_competitors: vec![entity_id(2, 0), entity_id(3, 0)],
            domain: ActionDomain::Production,
            effective_discount: Permille::new(400).unwrap(),
            pre_discount_motive: 700,
            post_discount_motive: 420,
        };
        let ranked = AgendaEntry {
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(GoalKind::Sleep),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score: discount.post_discount_motive,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: Some(discount.clone()),
            learned_opportunity_bonus: None,
            repair_memory_bonus: None,
            source_composite: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            partial_plan_segment: None,
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::Sleep),
                anchor: worldwake_core::OpportunityAnchor::None,
            },

            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };

        assert_eq!(ranked.competition_discount, Some(discount));
    }

    #[test]
    fn ranked_goal_supports_optional_source_reliability_discount() {
        let discount = SourceReliabilityDiscount {
            source_entity: entity_id(9, 0),
            commodity: CommodityKind::Bread,
            failure_ratio_permille: 500,
            pre_discount_motive: 700,
            post_discount_motive: 350,
            provenance_event_count: 0,
            most_recent_provenance_event: None,
        };
        let ranked = AgendaEntry {
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }),
                evidence_entities: BTreeSet::from([entity_id(9, 0)]),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score: discount.post_discount_motive,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: Some(discount.clone()),
            competition_discount: None,
            learned_opportunity_bonus: None,
            repair_memory_bonus: None,
            source_composite: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            partial_plan_segment: None,
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::Sleep),
                anchor: worldwake_core::OpportunityAnchor::None,
            },

            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };

        assert_eq!(ranked.source_reliability_discount, Some(discount));
    }

    #[test]
    fn crate_re_exports_the_canonical_shared_goal_identity() {
        let kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let key = GoalKey::from(kind);

        assert_eq!(key.kind, kind);
        assert_eq!(key.commodity, Some(CommodityKind::Water));
    }

    #[test]
    fn grounded_goal_roundtrips_through_bincode() {
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::TreatWounds {
                patient: entity_id(7, 1),
            }),
            evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
            evidence_places: BTreeSet::from([entity_id(10, 0)]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GoalOffer = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn ranked_goal_roundtrips_through_bincode() {
        let goal = AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(GoalKind::TreatWounds {
                    patient: entity_id(7, 1),
                }),
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(GoalKind::TreatWounds {
                    patient: entity_id(7, 1),
                }),
                evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
                evidence_places: BTreeSet::from([entity_id(10, 0)]),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score: 900,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            learned_opportunity_bonus: None,
            repair_memory_bonus: None,
            source_composite: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            partial_plan_segment: None,

            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: AgendaEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn ranked_goal_provenance_family_is_payload_aware() {
        assert_eq!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }
            .ranked_goal_provenance_family(),
            Some(RankedGoalProvenanceFamily::Drive)
        );
        assert_eq!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::Restock,
                quantity: AcquisitionQuantity::single(),
            }
            .ranked_goal_provenance_family(),
            None
        );
        assert_eq!(
            GoalKind::ReduceDanger.ranked_goal_provenance_family(),
            Some(RankedGoalProvenanceFamily::Danger)
        );
    }

    #[test]
    fn steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions() {
        let steal = GoalKind::StealItem {
            target_item: entity_id(9, 0),
        };
        assert!(steal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(
            steal
                .relevant_op_kinds()
                .contains(&PlannerOpKind::MoveCargo)
        );

        let accuse = GoalKind::Accuse {
            crime_register: entity_id(9, 0),
            accused: entity_id(10, 0),
            violation_id: ViolationId(2),
        };
        assert!(accuse.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(accuse.relevant_op_kinds().contains(&PlannerOpKind::Accuse));

        let punish = GoalKind::PunishAccused {
            office: entity_id(10, 0),
            accused: entity_id(11, 0),
            accusation_entry: RecordEntryId(2),
            punishment: PunishmentKind::Fine {
                commodity: CommodityKind::Coin,
                amount: Quantity(3),
            },
        };
        assert!(punish.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(punish.relevant_op_kinds().contains(&PlannerOpKind::Fine));
    }

    #[test]
    fn consume_goal_relevant_ops_include_consumption_and_pickup_only() {
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Consume));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::MoveCargo));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Harvest));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Craft));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Trade));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Attack));
    }

    #[test]
    fn reduce_danger_goal_relevant_ops_include_defense_leaf_options() {
        let goal = GoalKind::ReduceDanger;

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Defend));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Heal));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Attack));
    }

    #[test]
    fn engage_hostile_goal_relevant_ops_include_travel_and_attack() {
        let goal = GoalKind::EngageHostile {
            target: entity_id(4, 0),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[PlannerOpKind::Travel, PlannerOpKind::Attack]
        );
    }

    #[test]
    fn raid_target_goal_relevant_ops_include_travel_and_attack() {
        let goal = GoalKind::RaidTarget {
            target: entity_id(4, 1),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[PlannerOpKind::Travel, PlannerOpKind::Attack]
        );
    }

    #[test]
    fn search_for_missing_goal_relevant_ops_stay_reserved_to_search_surface() {
        let goal = GoalKind::SearchForMissing {
            subject: entity_id(4, 2),
            last_seen: Some(entity_id(7, 0)),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::AskAboutPerson,
                PlannerOpKind::SearchPlace,
            ]
        );
    }

    #[test]
    fn escort_to_safety_goal_relevant_ops_stay_reserved_to_escort_surface() {
        let goal = GoalKind::EscortToSafety {
            subject: entity_id(4, 4),
            destination: entity_id(9, 0),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[PlannerOpKind::Travel, PlannerOpKind::EscortToSafety]
        );
    }

    #[test]
    fn regroup_with_faction_goal_relevant_ops_are_travel_only() {
        let goal = GoalKind::RegroupWithFaction {
            faction: entity_id(4, 2),
        };

        assert_eq!(goal.relevant_op_kinds(), &[PlannerOpKind::Travel]);
    }

    #[test]
    fn share_belief_goal_relevant_ops_are_tell_only() {
        let goal = GoalKind::ShareBelief {
            listener: entity_id(4, 0),
            topic: TellTopic::EntityBelief {
                subject: entity_id(5, 0),
            },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };

        assert_eq!(goal.relevant_op_kinds(), &[PlannerOpKind::Tell]);
    }

    #[test]
    fn sleep_goal_observed_commodities_are_empty() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::Sleep.relevant_observed_commodities(&recipes),
            Some(BTreeSet::new())
        );
    }

    #[test]
    fn share_belief_goal_observed_commodities_are_empty() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::ShareBelief {
                listener: entity_id(6, 0),
                topic: TellTopic::EntityBelief {
                    subject: entity_id(7, 0),
                },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            }
            .relevant_observed_commodities(&recipes),
            Some(BTreeSet::new())
        );
    }

    #[test]
    fn share_belief_tell_step_is_a_progress_barrier() {
        let goal = GoalKind::ShareBelief {
            listener: entity_id(6, 0),
            topic: TellTopic::EntityBelief {
                subject: entity_id(7, 0),
            },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };
        let step = PlannedStep {
            def_id: ActionDefId(77),
            op_kind: PlannerOpKind::Tell,
            targets: vec![crate::PlanningEntityRef::Authoritative(entity_id(6, 0))],
            target_place: None,
            payload_override: None,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };

        assert!(goal.is_progress_barrier(&step));
    }

    #[test]
    fn move_cargo_goal_observed_commodities_track_goal_commodity_only() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: entity_id(5, 0),
            }
            .relevant_observed_commodities(&recipes),
            Some(BTreeSet::from([CommodityKind::Bread]))
        );
    }

    #[test]
    fn target_commodity_maps_supported_goal_rows() {
        let mut recipes = worldwake_sim::RecipeRegistry::new();
        let recipe_id = recipes.register(worldwake_sim::RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(1).unwrap(),
            ),
        });
        let patient = entity_id(8, 0);
        let destination = entity_id(9, 0);

        assert_eq!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }
            .target_commodity(&recipes),
            Some(CommodityKind::Water)
        );
        assert_eq!(
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }
            .target_commodity(&recipes),
            Some(CommodityKind::Bread)
        );
        assert_eq!(
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }
            .target_commodity(&recipes),
            Some(CommodityKind::Apple)
        );
        assert_eq!(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Medicine,
            }
            .target_commodity(&recipes),
            Some(CommodityKind::Medicine)
        );
        assert_eq!(
            GoalKind::MoveCargo {
                commodity: CommodityKind::Coin,
                destination,
            }
            .target_commodity(&recipes),
            Some(CommodityKind::Coin)
        );
        assert_eq!(
            GoalKind::TreatWounds { patient }.target_commodity(&recipes),
            Some(CommodityKind::Medicine)
        );
        assert_eq!(
            GoalKind::ProduceCommodity { recipe_id }.target_commodity(&recipes),
            Some(CommodityKind::Bread)
        );
        assert_eq!(
            GoalKind::FreeCarryCapacity.target_commodity(&recipes),
            Some(CommodityKind::Waste)
        );
        assert_eq!(GoalKind::Sleep.target_commodity(&recipes), None);
    }

    #[test]
    fn target_commodity_returns_none_for_missing_produce_recipe() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(999),
            }
            .target_commodity(&recipes),
            None
        );
    }

    #[test]
    fn free_carry_capacity_goal_observed_commodities_track_waste() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::FreeCarryCapacity.relevant_observed_commodities(&recipes),
            Some(BTreeSet::from([CommodityKind::Waste]))
        );
    }

    #[test]
    fn produce_goal_observed_commodities_include_recipe_inputs_and_outputs() {
        let mut recipes = worldwake_sim::RecipeRegistry::new();
        let recipe_id = recipes.register(worldwake_sim::RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(1).unwrap(),
            ),
        });

        assert_eq!(
            GoalKind::ProduceCommodity { recipe_id }.relevant_observed_commodities(&recipes),
            Some(BTreeSet::from([CommodityKind::Bread, CommodityKind::Grain]))
        );
    }

    #[test]
    fn missing_produce_recipe_falls_back_to_full_observed_commodity_tracking() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(999),
            }
            .relevant_observed_commodities(&recipes),
            None
        );
    }

    #[test]
    fn restock_goal_relevant_ops_include_trade_production_and_cargo() {
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Trade));
        assert!(
            goal.relevant_op_kinds()
                .contains(&PlannerOpKind::QueueForFacilityUse)
        );
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Harvest));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Craft));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::MoveCargo));
    }

    #[test]
    fn bury_goal_uses_bury_op_family() {
        let goal = GoalKind::BuryCorpse {
            corpse: entity_id(1, 0),
            burial_site: entity_id(2, 0),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[PlannerOpKind::QueueForFacilityUse, PlannerOpKind::Bury]
        );
    }

    #[test]
    fn loot_goal_uses_queue_then_loot_op_family() {
        let goal = GoalKind::LootCorpse {
            corpse: entity_id(1, 0),
        };

        assert_eq!(
            goal.relevant_op_kinds(),
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::QueueForFacilityUse,
                PlannerOpKind::Loot,
            ]
        );
    }

    #[test]
    fn move_cargo_satisfied_when_destination_stocked() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let bread = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at.insert(destination, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn move_cargo_not_satisfied_when_destination_understocked() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let bread = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at.insert(destination, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn move_cargo_satisfaction_is_destination_local() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let remote = entity_id(3, 0);
        let bread = entity_id(4, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, remote);
        view.effective_places.insert(bread, remote);
        view.entities_at.insert(remote, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination, remote]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn move_cargo_facility_destination_requires_facility_custody_not_carried_stock() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let stock_container = entity_id(4, 0);
        let bread = entity_id(5, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, facility, stock_container, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.kinds.insert(stock_container, EntityKind::Container);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(facility, destination);
        view.effective_places.insert(stock_container, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at
            .insert(destination, vec![actor, facility, stock_container, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable
            .extend([(actor, facility), (actor, stock_container), (actor, bread)]);
        view.stock_storage_policies.insert(
            facility,
            worldwake_core::StockStoragePolicy {
                stock_container,
                display_container: None,
            },
        );
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: facility,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn move_cargo_facility_destination_accepts_stock_in_container() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let stock_container = entity_id(4, 0);
        let bread = entity_id(5, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, facility, stock_container, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.kinds.insert(stock_container, EntityKind::Container);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(facility, destination);
        view.effective_places.insert(stock_container, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at
            .insert(destination, vec![actor, facility, stock_container, bread]);
        view.direct_containers.insert(bread, stock_container);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable
            .extend([(actor, facility), (actor, stock_container), (actor, bread)]);
        view.stock_storage_policies.insert(
            facility,
            worldwake_core::StockStoragePolicy {
                stock_container,
                display_container: None,
            },
        );
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: facility,
            }
            .is_satisfied(&state)
        );
    }

    struct TestBeliefView {
        current_tick: Tick,
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        controlled_quantities: BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
        controllable: BTreeSet<(EntityId, EntityId)>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        exposures: BTreeMap<EntityId, DeprivationExposure>,
        escalation_profiles: BTreeMap<EntityId, DriveEscalationProfile>,
        disposal_profiles: BTreeMap<EntityId, DisposalProfile>,
        trade_profiles: BTreeMap<EntityId, TradeDispositionProfile>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        listed_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        lot_sellers: BTreeMap<EntityId, EntityId>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        wash_basin_states: BTreeMap<EntityId, WashBasinState>,
        workstation_tags: BTreeMap<EntityId, WorkstationTag>,
        place_tags: BTreeMap<EntityId, BTreeSet<worldwake_core::PlaceTag>>,
        courage_values: BTreeMap<EntityId, Permille>,
        combat_profiles: BTreeMap<EntityId, Option<CombatProfile>>,
        consultation_speed_factors: BTreeMap<EntityId, Permille>,
        record_data: BTreeMap<EntityId, worldwake_core::RecordData>,
        known_entity_beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        belief_stores: BTreeMap<EntityId, AgentBeliefStore>,
        known_institutional_beliefs: BTreeMap<EntityId, Vec<BelievedInstitutionalClaim>>,
        epistemic_profiles: BTreeMap<EntityId, EpistemicDispositionProfile>,
        ask_witness_memories: BTreeMap<(EntityId, AskWitnessMemoryKey), AskWitnessMemory>,
        office_holder_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        force_controller_beliefs:
            BTreeMap<EntityId, InstitutionalBeliefRead<(Option<EntityId>, bool)>>,
        faction_rally_point_beliefs: BTreeMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
        support_declaration_beliefs:
            BTreeMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
        office_data_map: BTreeMap<EntityId, OfficeData>,
        stock_storage_policies: BTreeMap<EntityId, worldwake_core::StockStoragePolicy>,
    }

    impl Default for TestBeliefView {
        fn default() -> Self {
            Self {
                current_tick: Tick(0),
                alive: BTreeSet::new(),
                kinds: BTreeMap::new(),
                effective_places: BTreeMap::new(),
                entities_at: BTreeMap::new(),
                direct_possessions: BTreeMap::new(),
                direct_possessors: BTreeMap::new(),
                direct_containers: BTreeMap::new(),
                adjacent: BTreeMap::new(),
                lot_commodities: BTreeMap::new(),
                consumable_profiles: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                controlled_quantities: BTreeMap::new(),
                controllable: BTreeSet::new(),
                demand_memory: BTreeMap::new(),
                carry_capacities: BTreeMap::new(),
                entity_loads: BTreeMap::new(),
                needs: BTreeMap::new(),
                thresholds: BTreeMap::new(),
                exposures: BTreeMap::new(),
                escalation_profiles: BTreeMap::new(),
                disposal_profiles: BTreeMap::new(),
                trade_profiles: BTreeMap::new(),
                merchandise_profiles: BTreeMap::new(),
                listed_lots: BTreeMap::new(),
                lot_sellers: BTreeMap::new(),
                wounds: BTreeMap::new(),
                hostiles: BTreeMap::new(),
                resource_sources: BTreeMap::new(),
                wash_basin_states: BTreeMap::new(),
                workstation_tags: BTreeMap::new(),
                place_tags: BTreeMap::new(),
                courage_values: BTreeMap::new(),
                combat_profiles: BTreeMap::new(),
                consultation_speed_factors: BTreeMap::new(),
                record_data: BTreeMap::new(),
                known_entity_beliefs: BTreeMap::new(),
                belief_stores: BTreeMap::new(),
                known_institutional_beliefs: BTreeMap::new(),
                epistemic_profiles: BTreeMap::new(),
                ask_witness_memories: BTreeMap::new(),
                office_holder_beliefs: BTreeMap::new(),
                force_controller_beliefs: BTreeMap::new(),
                faction_rally_point_beliefs: BTreeMap::new(),
                support_declaration_beliefs: BTreeMap::new(),
                office_data_map: BTreeMap::new(),
                stock_storage_policies: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for TestBeliefView {
        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
                || <Self as worldwake_sim::InventoryBeliefView>::direct_possessor(self, entity)
                    == Some(actor)
                || self.controllable.contains(&(actor, entity))
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

    impl worldwake_sim::BelievedAuthorityView for TestBeliefView {
        fn believed_office_holder(
            &self,
            office: EntityId,
        ) -> worldwake_sim::BeliefRead<Option<EntityId>> {
            match self
                .office_holder_beliefs
                .get(&office)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
            {
                InstitutionalBeliefRead::Certain(holder) => {
                    worldwake_sim::BeliefRead::known_certain(holder, Tick(0))
                }
                InstitutionalBeliefRead::Conflicted(_) | InstitutionalBeliefRead::Unknown => {
                    worldwake_sim::BeliefRead::Unknown
                }
            }
        }
    }

    impl EntityBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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
            Some(MetabolismProfile::default())
        }

        fn disposal_profile(&self, agent: EntityId) -> Option<DisposalProfile> {
            self.disposal_profiles.get(&agent).copied()
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
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
        }

        fn place_has_tag(&self, place: EntityId, tag: worldwake_core::PlaceTag) -> bool {
            self.place_tags
                .get(&place)
                .is_some_and(|tags| tags.contains(&tag))
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }
    }

    impl TemporalBeliefView for TestBeliefView {
        fn current_tick(&self) -> Tick {
            self.current_tick
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            actor: EntityId,
            duration: &DurationExpr,
            targets: &[EntityId],
            payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            estimate_duration_from_beliefs(self, actor, duration, targets, payload)
        }
    }

    impl RuntimeBeliefView for TestBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for TestBeliefView {}

    impl worldwake_sim::SocialBeliefView for TestBeliefView {
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.known_entity_beliefs
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
            self.belief_stores.get(&agent)
        }

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
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

        fn ask_witness_memory(
            &self,
            actor: EntityId,
            key: &AskWitnessMemoryKey,
        ) -> Option<AskWitnessMemory> {
            self.ask_witness_memories.get(&(actor, *key)).cloned()
        }
    }

    impl worldwake_sim::PoliticalBeliefView for TestBeliefView {
        fn known_institutional_beliefs(&self, agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
            self.known_institutional_beliefs
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn record_data(&self, record: EntityId) -> Option<worldwake_core::RecordData> {
            self.record_data.get(&record).cloned()
        }

        fn office_data(&self, office: EntityId) -> Option<OfficeData> {
            self.office_data_map.get(&office).cloned()
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

        fn believed_faction_rally_point(
            &self,
            faction: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            self.faction_rally_point_beliefs
                .get(&faction)
                .cloned()
                .unwrap_or(InstitutionalBeliefRead::Unknown)
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

        fn believed_support_declarations_for_office(
            &self,
            office: EntityId,
        ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
            self.support_declaration_beliefs
                .iter()
                .filter_map(|(&(belief_office, supporter), read)| {
                    (belief_office == office).then_some((supporter, read.clone()))
                })
                .collect()
        }
    }

    impl worldwake_sim::CombatBeliefView for TestBeliefView {
        fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
            if let Some(override_val) = self.combat_profiles.get(&agent) {
                return *override_val;
            }
            Some(CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(6).unwrap(),
                NonZeroU32::new(10).unwrap(),
            ))
        }

        fn courage(&self, agent: EntityId) -> Option<Permille> {
            self.courage_values.get(&agent).copied()
        }

        fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
            self.consultation_speed_factors.get(&agent).copied()
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
    }

    impl worldwake_sim::EconomicBeliefView for TestBeliefView {
        fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
            self.trade_profiles.get(&agent).cloned()
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.controlled_quantities
                .get(&(actor, place, commodity))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn local_controlled_lots_for(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            let mut entities = self.entities_at(place);
            entities.extend(
                <Self as worldwake_sim::InventoryBeliefView>::direct_possessions(self, actor),
            );
            entities.sort();
            entities.dedup();
            entities
                .into_iter()
                .filter(|entity| {
                    <Self as worldwake_sim::InventoryBeliefView>::item_lot_commodity(self, *entity)
                        == Some(commodity)
                })
                .filter(|entity| self.can_control(actor, *entity))
                .collect()
        }

        fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.listed_lots
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }

        fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
            self.lot_sellers.get(&lot).copied()
        }

        fn has_sale_listing(&self, lot: EntityId) -> bool {
            self.lot_sellers.contains_key(&lot)
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

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl worldwake_sim::FacilityBeliefView for TestBeliefView {
        fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
            self.workstation_tags.get(&entity).copied()
        }

        fn stock_storage_policy(
            &self,
            facility: EntityId,
        ) -> Option<worldwake_core::StockStoragePolicy> {
            self.stock_storage_policies.get(&facility).cloned()
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn wash_basin_state(&self, entity: EntityId) -> Option<WashBasinState> {
            self.wash_basin_states.get(&entity).copied()
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

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|s| s.commodity == commodity)
                })
                .collect()
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

    fn base_view() -> (TestBeliefView, EntityId, EntityId) {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let bread = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, seller, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityConsumableProfile::new(NonZeroU32::new(2).unwrap(), pm(250), pm(0), pm(0)),
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(700), pm(0), pm(700), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.trade_profiles
            .insert(seller, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: None,
            },
        );
        let seller_lot = entity(30);
        view.alive.insert(seller_lot);
        view.kinds.insert(seller_lot, EntityKind::ItemLot);
        view.effective_places.insert(seller_lot, town);
        view.entities_at.get_mut(&town).unwrap().push(seller_lot);
        view.direct_possessors.insert(seller_lot, seller);
        view.direct_possessions
            .entry(seller)
            .or_default()
            .push(seller_lot);
        view.lot_commodities
            .insert(seller_lot, CommodityKind::Bread);
        view.listed_lots
            .insert((town, CommodityKind::Bread), vec![seller_lot]);
        view.lot_sellers.insert(seller_lot, seller);
        (view, actor, seller)
    }

    fn free_carry_capacity_view() -> (TestBeliefView, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let place = entity(10);
        let waste_lot = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, waste_lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(waste_lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(waste_lot, place);
        view.entities_at.insert(place, vec![actor, waste_lot]);
        view.direct_possessions.insert(actor, vec![waste_lot]);
        view.direct_possessors.insert(waste_lot, actor);
        view.lot_commodities.insert(waste_lot, CommodityKind::Waste);
        view.commodity_quantities
            .insert((actor, CommodityKind::Waste), Quantity(9));
        view.commodity_quantities
            .insert((waste_lot, CommodityKind::Waste), Quantity(9));
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(waste_lot, LoadUnits(9));
        (view, actor, place, waste_lot)
    }

    #[test]
    fn acquire_goal_builds_trade_payload_override_from_goal_semantics() {
        let (view, actor, seller) = base_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let def = ActionDef {
            id: ActionDefId(9),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[seller], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                sale_lot: entity(30),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(3),
                requested_quantity: Quantity(1),
            }))
        );
    }

    #[test]
    fn move_cargo_pickup_builds_exact_transport_quantity_payload() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![actor, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(5));
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(0));
        view.direct_possessions.insert(actor, Vec::new());
        view.carry_capacities.insert(actor, LoadUnits(2));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([origin, destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        };
        let def = ActionDef {
            id: ActionDefId(9),
            name: "pick_up".to_string(),
            domain: ActionDomain::Transport,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[bread], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(2),
            }))
        );
    }

    #[test]
    fn share_belief_goal_builds_tell_payload_override() {
        let actor = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, listener, subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Facility);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(subject, place);
        view.entities_at
            .insert(place, vec![actor, listener, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };
        let def = ActionDef {
            id: ActionDefId(10),
            name: "tell".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Tell,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[listener], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Tell(TellActionPayload {
                listener,
                topic: TellTopic::EntityBelief { subject },
            }))
        );
    }

    #[test]
    fn share_belief_goal_reuses_matching_affordance_payload() {
        let actor = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, listener, subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Facility);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(subject, place);
        view.entities_at
            .insert(place, vec![actor, listener, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let topic = TellTopic::EntityBelief { subject };
        let goal = GoalKind::ShareBelief {
            listener,
            topic,
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };
        let def = ActionDef {
            id: ActionDefId(10),
            name: "tell".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Tell,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };
        let affordance_payload = ActionPayload::Tell(TellActionPayload { listener, topic });

        let payload = goal
            .build_payload_override(
                Some(&affordance_payload),
                &state,
                &[listener],
                &def,
                &semantics,
            )
            .unwrap();

        assert_eq!(payload, Some(affordance_payload));
    }

    #[test]
    fn share_belief_goal_rejects_mismatched_affordance_topic() {
        let actor = entity(1);
        let listener = entity(2);
        let right_subject = entity(3);
        let wrong_subject = entity(4);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, listener, right_subject, wrong_subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.kinds.insert(right_subject, EntityKind::Facility);
        view.kinds.insert(wrong_subject, EntityKind::Facility);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(right_subject, place);
        view.effective_places.insert(wrong_subject, place);
        view.entities_at
            .insert(place, vec![actor, listener, right_subject, wrong_subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener, right_subject, wrong_subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief {
                subject: right_subject,
            },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        };
        let def = ActionDef {
            id: ActionDefId(10),
            name: "tell".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Tell,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };
        let affordance_payload = ActionPayload::Tell(TellActionPayload {
            listener,
            topic: TellTopic::EntityBelief {
                subject: wrong_subject,
            },
        });

        let result = goal.build_payload_override(
            Some(&affordance_payload),
            &state,
            &[listener],
            &def,
            &semantics,
        );

        assert_eq!(result, Err(GoalPayloadOverrideError::UnsupportedGoal));
    }

    fn ask_witness_action_def() -> ActionDef {
        ActionDef {
            id: ActionDefId(30),
            name: "ask_witness".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        }
    }

    #[test]
    fn ask_witness_goal_builds_payload_override_for_entity_belief_topic() {
        let actor = entity(1);
        let witness = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, witness, subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.entities_at.insert(place, vec![actor, witness]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::AskWitness {
            witness,
            topic: TellTopic::EntityBelief { subject },
        };
        let def = ask_witness_action_def();
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::AskWitness,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[witness], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::AskWitness(AskWitnessPayload {
                target: witness,
                topic_entity: Some(subject),
                topic_commodity: None,
            }))
        );
    }

    #[test]
    fn ask_witness_goal_rejects_unsupported_topic_payload_override() {
        let actor = entity(1);
        let witness = entity(2);
        let office = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, witness, office, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.entities_at.insert(place, vec![actor, witness]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, office]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let def = ask_witness_action_def();
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::AskWitness,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };
        let institutional = GoalKind::AskWitness {
            witness,
            topic: TellTopic::InstitutionalClaim {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(witness),
                    effective_tick: Tick(1),
                },
            },
        };
        let social = GoalKind::AskWitness {
            witness,
            topic: TellTopic::SocialObservation {
                observation: SocialObservation {
                    detail: SocialObservationDetail::WitnessedCooperation {
                        actor,
                        counterpart: witness,
                    },
                    place,
                    observed_tick: Tick(1),
                    source: PerceptionSource::DirectObservation,
                },
            },
        };

        assert_eq!(
            institutional.build_payload_override(None, &state, &[witness], &def, &semantics),
            Err(GoalPayloadOverrideError::UnsupportedTopic)
        );
        assert_eq!(
            social.build_payload_override(None, &state, &[witness], &def, &semantics),
            Err(GoalPayloadOverrideError::UnsupportedTopic)
        );
    }

    fn ask_witness_goal_satisfied_with_report_age(
        staleness_ticks: u64,
        profile: EpistemicDispositionProfile,
    ) -> bool {
        let actor = entity(1);
        let witness = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let current_tick = Tick(100);
        let observed_tick = Tick(current_tick.0.saturating_sub(staleness_ticks));
        let mut view = TestBeliefView {
            current_tick,
            ..Default::default()
        };
        view.alive.extend([actor, witness, subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.entities_at.insert(place, vec![actor, witness]);
        view.epistemic_profiles.insert(actor, profile);
        let mut belief = BelievedEntityState::single_observation_defaults(
            observed_tick,
            PerceptionSource::Report {
                from: witness,
                chain_len: 1,
            },
        );
        belief.believed_kind = Some(EntityKind::Agent);
        belief.last_known_place = Some(place);
        view.known_entity_beliefs
            .insert(actor, vec![(subject, belief)]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::AskWitness {
            witness,
            topic: TellTopic::EntityBelief { subject },
        };

        goal.is_satisfied(&state)
    }

    #[test]
    fn ask_witness_goal_is_satisfied_by_recent_below_threshold_report() {
        let profile = EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: pm(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
            witness_recency_preference: pm(500),
        };

        assert!(ask_witness_goal_satisfied_with_report_age(35, profile));
    }

    #[test]
    fn ask_witness_goal_rejects_stale_below_threshold_report() {
        let profile = EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: pm(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
            witness_recency_preference: pm(500),
        };

        assert!(!ask_witness_goal_satisfied_with_report_age(42, profile));
    }

    #[test]
    fn ask_witness_goal_is_satisfied_by_matching_report_above_threshold() {
        let profile = EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: pm(200),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
            witness_recency_preference: pm(100),
        };

        assert!(ask_witness_goal_satisfied_with_report_age(42, profile));
    }

    #[test]
    fn investigate_goal_builds_investigate_payload_override() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([place]), 1);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(7),
            place,
        };
        let def = ActionDef {
            id: ActionDefId(11),
            name: "investigate".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Investigate,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[place], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id: worldwake_core::ViolationId(7),
            }))
        );
    }

    #[test]
    fn search_for_missing_builds_search_place_payload_override_when_colocated() {
        let actor = entity(1);
        let place = entity(10);
        let subject = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(subject, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SearchForMissing {
            subject,
            last_seen: Some(place),
        };
        let def = ActionDef {
            id: ActionDefId(26),
            name: "search_place".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::SearchPlace,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[place], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
                subject,
            }))
        );
    }

    #[test]
    fn search_for_missing_rejects_ask_about_person_payload_when_last_seen_is_local() {
        let actor = entity(1);
        let place = entity(10);
        let witness = entity(11);
        let subject = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, witness, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.effective_places.insert(subject, place);
        view.entities_at
            .insert(place, vec![actor, witness, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SearchForMissing {
            subject,
            last_seen: Some(place),
        };
        let def = ActionDef {
            id: ActionDefId(27),
            name: "ask_about_person".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::AskAboutPerson,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let result = goal.build_payload_override(None, &state, &[witness], &def, &semantics);

        assert_eq!(result, Err(GoalPayloadOverrideError::UnsupportedGoal));
    }

    #[test]
    fn search_for_missing_rejects_ask_about_person_payload_during_ask_memory_retention() {
        let actor = entity(1);
        let place = entity(10);
        let remote = entity(11);
        let witness = entity(12);
        let subject = entity(13);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, remote, witness, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.effective_places.insert(subject, remote);
        view.entities_at.insert(place, vec![actor, witness]);
        view.entities_at.insert(remote, vec![subject]);
        view.epistemic_profiles.insert(
            actor,
            EpistemicDispositionProfile {
                ask_memory_retention_ticks: 10,
                ..EpistemicDispositionProfile::default()
            },
        );
        let memory_key = AskWitnessMemoryKey {
            counterparty: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let mut store = AgentBeliefStore::new();
        store.record_asked_witness(
            memory_key,
            AskWitnessMemory {
                asked_tick: Tick(4),
            },
        );
        view.belief_stores.insert(actor, store);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject]),
            &BTreeSet::from([place, remote]),
            6,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SearchForMissing {
            subject,
            last_seen: Some(remote),
        };
        let def = ActionDef {
            id: ActionDefId(29),
            name: "ask_about_person".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::AskAboutPerson,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let result = goal.build_payload_override(None, &state, &[witness], &def, &semantics);

        assert_eq!(result, Err(GoalPayloadOverrideError::UnsupportedGoal));
    }

    #[test]
    fn search_for_missing_marks_ask_about_person_unavailable_when_last_seen_is_local() {
        let actor = entity(1);
        let place = entity(10);
        let subject = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(subject, place);
        view.entities_at.insert(place, vec![actor, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SearchForMissing {
            subject,
            last_seen: Some(place),
        };

        assert!(!goal.candidate_is_available(&state, PlannerOpKind::AskAboutPerson));
        assert!(goal.candidate_is_available(&state, PlannerOpKind::SearchPlace));
    }

    #[test]
    fn search_for_missing_keeps_ask_about_person_payload_when_last_seen_is_remote() {
        let actor = entity(1);
        let place = entity(10);
        let remote = entity(11);
        let witness = entity(12);
        let subject = entity(13);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, remote, witness, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(witness, place);
        view.effective_places.insert(subject, remote);
        view.entities_at.insert(place, vec![actor, witness]);
        view.entities_at.insert(remote, vec![subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject]),
            &BTreeSet::from([place, remote]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SearchForMissing {
            subject,
            last_seen: Some(remote),
        };
        let def = ActionDef {
            id: ActionDefId(28),
            name: "ask_about_person".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::AskAboutPerson,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[witness], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::AskAboutPerson(AskAboutPersonActionPayload {
                target: witness,
                subject,
            }))
        );
    }

    #[test]
    fn report_missing_builds_payload_override_from_expectation_id() {
        let actor = entity(1);
        let place = entity(10);
        let subject = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, subject]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(subject, EntityKind::Agent);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ReportMissing {
            subject,
            to_office: None,
            expectation_id: Some(worldwake_core::ExpectationId(7)),
        };
        let def = ActionDef {
            id: ActionDefId(37),
            name: "report_missing".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::ReportMissing,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[place], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::ReportMissing(ReportMissingActionPayload {
                expectation_id: worldwake_core::ExpectationId(7),
            }))
        );
    }

    #[test]
    fn accuse_goal_builds_accuse_payload_override() {
        let actor = entity(1);
        let accused = entity(10);
        let place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, accused, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(accused, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(accused, place);
        view.entities_at.insert(place, vec![actor, accused]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([accused]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::Accuse {
            crime_register: entity(9),
            accused,
            violation_id: worldwake_core::ViolationId(9),
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "accuse".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::SpecificEntity(accused)],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Accuse,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[accused], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Accuse(AccuseActionPayload {
                violation_id: worldwake_core::ViolationId(9),
            }))
        );
    }

    #[test]
    fn punish_goal_builds_case_bound_payload_override() {
        let actor = entity(1);
        let accused = entity(10);
        let office = entity(11);
        let faction = entity(12);
        let place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, accused, office, faction, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(accused, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(faction, EntityKind::Faction);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(accused, place);
        view.entities_at.insert(place, vec![actor, accused]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([accused, office, faction]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::PunishAccused {
            office,
            accused,
            accusation_entry: RecordEntryId(11),
            punishment: PunishmentKind::Exile {
                from_faction: faction,
            },
        };
        let def = ActionDef {
            id: ActionDefId(13),
            name: "exile".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::SpecificEntity(accused)],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Exile,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[accused], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Punish(PunishActionPayload {
                office,
                accusation_entry: RecordEntryId(11),
                punishment: PunishmentKind::Exile {
                    from_faction: faction
                },
            }))
        );
    }

    #[test]
    fn grounded_goal_synthesizes_tell_root_targets_only_when_colocated() {
        let place = entity(7);
        let listener = entity(8);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief {
                    subject: entity(20),
                },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "tell".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Tell,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(place)),
            RootCandidateSynthesis::Targets(vec![listener])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(entity(99))),
            RootCandidateSynthesis::NoSynthesisPath
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_accuse_root_targets_from_goal_identity() {
        let accused = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::Accuse {
                crime_register: entity(9),
                accused,
                violation_id: worldwake_core::ViolationId(9),
            }),
            evidence_entities: BTreeSet::from([accused]),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "accuse".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::SpecificEntity(entity(999))],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Accuse,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::Targets(vec![accused])
        );
    }

    #[test]
    fn grounded_goal_synthesizes_claim_bounty_root_targets_from_goal_identity() {
        let bounty = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
            key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
            evidence_entities: BTreeSet::from([bounty]),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "claim_bounty".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::SpecificEntity(entity(999))],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::ClaimBounty,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::Targets(vec![bounty])
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_fine_root_targets_from_remote_evidence() {
        let actor_place = entity(10);
        let remote_place = entity(11);
        let accused = entity(12);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::PunishAccused {
                office: entity(13),
                accused,
                accusation_entry: RecordEntryId(14),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(3),
                },
            }),
            evidence_entities: BTreeSet::from([accused]),
            evidence_places: BTreeSet::from([remote_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "fine".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Fine,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(actor_place)),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_exile_root_targets_from_remote_evidence() {
        let actor_place = entity(10);
        let remote_place = entity(11);
        let accused = entity(12);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::PunishAccused {
                office: entity(13),
                accused,
                accusation_entry: RecordEntryId(14),
                punishment: PunishmentKind::Exile {
                    from_faction: entity(15),
                },
            }),
            evidence_entities: BTreeSet::from([accused]),
            evidence_places: BTreeSet::from([remote_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "exile".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Exile,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(actor_place)),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_escort_root_targets_from_remote_evidence() {
        let actor_place = entity(10);
        let remote_place = entity(11);
        let subject = entity(12);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::EscortToSafety {
                subject,
                destination: entity(13),
            }),
            evidence_entities: BTreeSet::from([subject]),
            evidence_places: BTreeSet::from([remote_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "escort_to_safety".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::EscortToSafety,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(actor_place)),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_post_notice_root_targets_when_colocated_with_posting_place() {
        let posting_place = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(posting_place),
            key: GoalKey::from(GoalKind::PostNotice {
                posting: ArtifactPostingContext {
                    posting_place,
                    issuing_authority: None,
                    expires_at: Some(Tick(5)),
                    jurisdiction: Some(posting_place),
                },
                topic: NoticeTopic::ThreatWarning {
                    place: posting_place,
                },
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([posting_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(13),
            name: "post_notice".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::PostNotice,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(posting_place)),
            RootCandidateSynthesis::Targets(vec![posting_place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(entity(99))),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_post_bounty_root_targets_when_colocated_with_posting_place() {
        let posting_place = entity(10);
        let target = entity(20);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(posting_place),
            key: GoalKey::from(GoalKind::PostBounty {
                posting: ArtifactPostingContext {
                    posting_place,
                    issuing_authority: None,
                    expires_at: Some(Tick(6)),
                    jurisdiction: Some(posting_place),
                },
                terms: BountyTerms {
                    target: BountyTarget::EliminateEntity { target },
                    proof_requirement: ProofRequirement::SelfReport,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(3),
                    reward_source: RewardSource::PersonalFunds { issuer: entity(1) },
                    claim_place: posting_place,
                },
            }),
            evidence_entities: BTreeSet::from([target]),
            evidence_places: BTreeSet::from([posting_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(14),
            name: "post_bounty".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::PostBounty,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(posting_place)),
            RootCandidateSynthesis::Targets(vec![posting_place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(entity(99))),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_establish_camp_root_targets_only_when_colocated() {
        let rally_place = entity(14);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(rally_place),
            key: GoalKey::from(GoalKind::EstablishBanditCamp {
                faction: entity(15),
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([rally_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "establish_camp".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::EstablishCamp,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(rally_place)),
            RootCandidateSynthesis::Targets(vec![rally_place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(entity(99))),
            RootCandidateSynthesis::NoSynthesisPath
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_trade_root_targets_from_local_single_evidence_entity() {
        let market = entity(10);
        let seller = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([market]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: true,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(market)),
            RootCandidateSynthesis::Targets(vec![seller])
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_trade_root_targets_from_remote_evidence() {
        let actor_place = entity(10);
        let remote_market = entity(12);
        let seller = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([remote_market]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: true,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(actor_place)),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_harvest_root_targets_from_remote_evidence() {
        let actor_place = entity(10);
        let remote_orchard = entity(12);
        let orchard_row = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            evidence_entities: BTreeSet::from([orchard_row]),
            evidence_places: BTreeSet::from([remote_orchard]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(13),
            name: "harvest:apple".to_string(),
            domain: ActionDomain::Production,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Facility,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Harvest,
            may_appear_mid_plan: false,
            is_materialization_barrier: true,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(actor_place)),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_trade_root_targets_from_ambiguous_evidence() {
        let market = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            evidence_entities: BTreeSet::from([entity(11), entity(12)]),
            evidence_places: BTreeSet::from([market]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: true,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(market)),
            RootCandidateSynthesis::TargetDerivationFailed
        );
    }

    #[test]
    fn grounded_goal_reports_unsupported_trade_synthesis_for_unrelated_goal() {
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::Sleep),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: true,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::UnsupportedGoalOp
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_attack_root_targets_for_raid_goal() {
        let target = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(target),
            key: GoalKey::from(GoalKind::RaidTarget { target }),
            evidence_entities: BTreeSet::from([target]),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Attack,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_does_not_synthesize_attack_root_targets_for_engage_hostile_goal() {
        let target = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(target),
            key: GoalKey::from(GoalKind::EngageHostile { target }),
            evidence_entities: BTreeSet::from([target]),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(13),
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Attack,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_investigate_root_targets_only_when_colocated() {
        let place = entity(10);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(place),
            key: GoalKey::from(GoalKind::InvestigateViolation {
                violation_id: worldwake_core::ViolationId(7),
                place,
            }),
            evidence_entities: BTreeSet::from([entity(11)]),
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(25),
            name: "investigate".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Investigate,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(place)),
            RootCandidateSynthesis::Targets(vec![place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(entity(99))),
            RootCandidateSynthesis::NoSynthesisPath
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_search_place_root_targets_only_when_colocated() {
        let place = entity(10);
        let subject = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(place),
            key: GoalKey::from(GoalKind::SearchForMissing {
                subject,
                last_seen: Some(place),
            }),
            evidence_entities: BTreeSet::from([subject]),
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(27),
            name: "search_place".to_string(),
            domain: ActionDomain::Epistemic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::SearchPlace,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(place)),
            RootCandidateSynthesis::Targets(vec![place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn grounded_goal_synthesizes_report_missing_root_targets_when_local_and_unbound() {
        let place = entity(10);
        let subject = entity(11);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(subject),
            key: GoalKey::from(GoalKind::ReportMissing {
                subject,
                to_office: None,
                expectation_id: Some(worldwake_core::ExpectationId(9)),
            }),
            evidence_entities: BTreeSet::from([subject]),
            evidence_places: BTreeSet::from([place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let def = ActionDef {
            id: ActionDefId(28),
            name: "report_missing".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::ReportMissing,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, Some(place)),
            RootCandidateSynthesis::Targets(vec![place])
        );
        assert_eq!(
            goal.synthesized_root_candidate_targets(&def, semantics, None),
            RootCandidateSynthesis::NoSynthesisPath
        );
    }

    #[test]
    fn investigate_goal_rejects_mismatched_affordance_payload() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.entities_at.insert(place, vec![actor]);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([place]), 1);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(7),
            place,
        };
        let def = ActionDef {
            id: ActionDefId(12),
            name: "investigate".to_string(),
            domain: ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: vec![worldwake_sim::TargetSpec::ActorPlace],
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Investigate,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let result = goal.build_payload_override(
            Some(&ActionPayload::Investigate(InvestigateActionPayload {
                violation_id: worldwake_core::ViolationId(8),
            })),
            &state,
            &[place],
            &def,
            &semantics,
        );

        assert_eq!(result, Err(GoalPayloadOverrideError::UnsupportedGoal));
    }

    #[test]
    fn is_satisfied_acquire_commodity_below_desired_min() {
        let (mut view, actor, _seller) = base_view();
        let bread_lot = entity(20);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(2));
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(2));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        let quantity = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(5).unwrap(),
            desired_target: std::num::NonZeroU16::new(5).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(200).unwrap(),
        };

        let self_consume = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity,
        };
        let restock = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::Restock,
            quantity,
        };

        assert!(
            !self_consume.is_satisfied(&state),
            "SelfConsume with desired_min=5 should not be satisfied with 2 units in possession"
        );
        assert!(
            !restock.is_satisfied(&state),
            "Restock with desired_min=5 should not be satisfied with controlled_commodity_quantity=2"
        );
    }

    #[test]
    fn is_satisfied_acquire_commodity_at_desired_min() {
        let (mut view, actor, _seller) = base_view();
        let bread_lot = entity(20);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(5));
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(5));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        let quantity = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(5).unwrap(),
            desired_target: std::num::NonZeroU16::new(7).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(200).unwrap(),
        };

        let self_consume = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity,
        };
        let restock = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::Restock,
            quantity,
        };

        assert!(
            self_consume.is_satisfied(&state),
            "SelfConsume with desired_min=5 should be satisfied with 5 units in possession"
        );
        assert!(
            restock.is_satisfied(&state),
            "Restock with desired_min=5 should be satisfied with controlled_commodity_quantity=5"
        );
    }

    #[test]
    fn consume_goal_satisfaction_is_owned_by_goal_model() {
        let (mut view, actor, _seller) = base_view();
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        let hungry_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let hungry_state = PlanningState::new(&hungry_snapshot);
        assert!(!goal.is_satisfied(&hungry_state));

        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(400), pm(0), pm(700), pm(0), pm(0)),
        );
        let low_band_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let low_band_state = PlanningState::new(&low_band_snapshot);
        assert!(
            !goal.is_satisfied(&low_band_state),
            "ConsumeOwnedCommodity must remain unsatisfied in the [low, medium) hunger band so search cannot return a zero-step root success"
        );

        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(100), pm(0), pm(700), pm(0), pm(0)),
        );
        let satiated_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let satiated_state = PlanningState::new(&satiated_snapshot);
        assert!(goal.is_satisfied(&satiated_state));
    }

    #[test]
    fn consume_goal_remains_unsatisfied_when_multi_relief_food_can_still_reduce_thirst() {
        let (mut view, actor, _seller) = base_view();
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Apple,
        };

        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(184), pm(586), pm(700), pm(0), pm(0)),
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let state = PlanningState::new(&snapshot);

        assert!(
            !goal.is_satisfied(&state),
            "Apple still relieves thirst in this state, so ConsumeOwnedCommodity(Apple) must not clear at the root node"
        );
    }

    #[test]
    fn self_care_goals_remain_unsatisfied_in_low_band() {
        let (mut view, actor, _seller) = base_view();
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(
                pm(0),
                pm(0),
                thresholds.fatigue.low(),
                thresholds.bladder.low(),
                thresholds.dirtiness.low(),
            ),
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::Sleep.is_satisfied(&state),
            "Sleep must remain unsatisfied in the [low, medium) fatigue band"
        );
        assert!(
            !GoalKind::Relieve.is_satisfied(&state),
            "Relieve must remain unsatisfied in the [low, medium) bladder band"
        );
        assert!(
            !GoalKind::Wash.is_satisfied(&state),
            "Wash must remain unsatisfied in the [low, medium) dirtiness band"
        );

        view.needs.insert(
            actor,
            HomeostaticNeeds::new(
                pm(0),
                pm(0),
                thresholds.fatigue.low().saturating_sub(pm(1)),
                thresholds.bladder.low().saturating_sub(pm(1)),
                thresholds.dirtiness.low().saturating_sub(pm(1)),
            ),
        );
        let relieved_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let relieved_state = PlanningState::new(&relieved_snapshot);

        assert!(GoalKind::Sleep.is_satisfied(&relieved_state));
        assert!(GoalKind::Relieve.is_satisfied(&relieved_state));
        assert!(GoalKind::Wash.is_satisfied(&relieved_state));
    }

    #[test]
    fn progress_barrier_semantics_move_with_goal_model() {
        let acquire_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let sleep_goal = GoalKind::Sleep;
        let sleep_step = PlannedStep {
            def_id: ActionDefId(2),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Sleep,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        let barrier_step = PlannedStep {
            def_id: ActionDefId(1),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Harvest,
            estimated_ticks: 3,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };

        assert!(acquire_goal.is_progress_barrier(&barrier_step));
        assert!(sleep_goal.is_progress_barrier(&sleep_step));
        assert!(!sleep_goal.is_progress_barrier(&barrier_step));
    }

    #[test]
    fn grounded_goal_epistemic_subjects_extract_stale_subjects_from_originating_goal_evidence() {
        let actor = entity(1);
        let source = entity(2);
        let town = entity(10);
        let remote = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, source, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(source, EntityKind::Facility);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(source, remote);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(remote, vec![source]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                source,
                believed_entity_state_at(
                    remote,
                    Tick(0),
                    Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(4),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    }),
                ),
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([source]),
            &BTreeSet::from([town, remote]),
            2,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(remote),
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::from([source]),
            evidence_places: BTreeSet::from([remote]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        assert_eq!(
            grounded_goal_epistemic_subjects(&goal, &state),
            vec![EpistemicSubject::SupplyAvailability {
                commodity: CommodityKind::Bread,
                source,
                place: remote,
            }]
        );
    }

    #[test]
    fn grounded_goal_epistemic_subjects_skip_accuse_goals() {
        let actor = entity(1);
        let accused = entity(2);
        let register = entity(3);
        let theft_place = entity(10);
        let hall = entity(11);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, accused, register, theft_place, hall]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(accused, EntityKind::Agent);
        view.kinds.insert(register, EntityKind::Record);
        view.kinds.insert(theft_place, EntityKind::Place);
        view.kinds.insert(hall, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, hall);
        view.effective_places.insert(accused, theft_place);
        view.effective_places.insert(register, hall);
        view.entities_at.insert(hall, vec![actor, register]);
        view.entities_at.insert(theft_place, vec![accused]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                accused,
                believed_entity_state_at(theft_place, Tick(0), None),
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([accused, register]),
            &BTreeSet::from([theft_place, hall]),
            2,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(accused),
            key: GoalKey::from(GoalKind::Accuse {
                crime_register: register,
                accused,
                violation_id: ViolationId(7),
            }),
            evidence_entities: BTreeSet::from([accused, register]),
            evidence_places: BTreeSet::from([theft_place, hall]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        assert!(
            grounded_goal_epistemic_subjects(&goal, &state).is_empty(),
            "Accuse should not inherit stale-evidence travel barriers from witness evidence"
        );
    }

    #[test]
    fn grounded_goal_epistemic_subjects_skip_actor_self_evidence() {
        let actor = entity(1);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(actor, believed_entity_state_at(town, Tick(0), None))],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([actor]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(town),
            key: GoalKey::from(GoalKind::Sleep),
            evidence_entities: BTreeSet::from([actor]),
            evidence_places: BTreeSet::from([town]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        assert!(
            grounded_goal_epistemic_subjects(&goal, &state).is_empty(),
            "Self-care goals grounded in actor self-evidence must not create witness-refresh barriers"
        );
    }

    #[test]
    fn grounded_goal_epistemic_barrier_matches_only_matching_payloads() {
        let actor = entity(1);
        let witness = entity(2);
        let source = entity(3);
        let town = entity(10);
        let remote = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, witness, source, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(source, EntityKind::Facility);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(witness, town);
        view.effective_places.insert(source, remote);
        view.entities_at.insert(town, vec![actor, witness]);
        view.entities_at.insert(remote, vec![source]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                source,
                believed_entity_state_at(
                    remote,
                    Tick(0),
                    Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(4),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    }),
                ),
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, source]),
            &BTreeSet::from([town, remote]),
            2,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(remote),
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::from([source]),
            evidence_places: BTreeSet::from([remote]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let subjects = grounded_goal_epistemic_subjects(&goal, &state);
        assert!(grounded_goal_matches_epistemic_barrier(
            &subjects,
            PlannerOpKind::Travel,
            &[remote],
            None,
        ));
        assert!(grounded_goal_matches_epistemic_barrier(
            &subjects,
            PlannerOpKind::AskWitness,
            &[witness],
            Some(&ActionPayload::AskWitness(AskWitnessPayload {
                target: witness,
                topic_entity: Some(source),
                topic_commodity: Some(CommodityKind::Bread),
            })),
        ));
        assert!(!grounded_goal_matches_epistemic_barrier(
            &subjects,
            PlannerOpKind::AskWitness,
            &[witness],
            Some(&ActionPayload::AskWitness(AskWitnessPayload {
                target: witness,
                topic_entity: None,
                topic_commodity: Some(CommodityKind::Apple),
            })),
        ));
        assert!(!grounded_goal_matches_epistemic_barrier(
            &subjects,
            PlannerOpKind::Travel,
            &[town],
            None,
        ));
    }

    #[test]
    fn acquire_self_consume_goal_is_not_satisfied_by_aggregate_quantity_without_held_lot() {
        let (view, actor, _seller) = base_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let state = PlanningState::new(&snapshot).with_commodity_quantity(
            actor,
            CommodityKind::Apple,
            Quantity(1),
        );
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };

        assert!(
            !goal.is_satisfied(&state),
            "self-consume acquisition must require a concrete held commodity lot, not only aggregate quantity"
        );
    }

    #[test]
    fn queue_for_facility_use_is_progress_barrier_for_exclusive_goal_families() {
        let queue_step = PlannedStep {
            def_id: ActionDefId(7),
            targets: Vec::new(),
            target_place: None,
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: ActionDefId(19),
                },
            )),
            op_kind: PlannerOpKind::QueueForFacilityUse,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };

        assert!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::Restock,
                quantity: AcquisitionQuantity::single(),
            }
            .is_progress_barrier(&queue_step)
        );
        assert!(GoalKind::LootCorpse { corpse: entity(41) }.is_progress_barrier(&queue_step));
        assert!(
            GoalKind::BuryCorpse {
                corpse: entity(41),
                burial_site: entity(42),
            }
            .is_progress_barrier(&queue_step)
        );
        assert!(
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0),
            }
            .is_progress_barrier(&queue_step)
        );
        assert!(!GoalKind::Sleep.is_progress_barrier(&queue_step));
    }

    #[test]
    fn political_goals_expose_political_op_families() {
        assert_eq!(
            GoalKind::ClaimOffice { office: entity(40) }.relevant_op_kinds(),
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::ConsultRecord,
                PlannerOpKind::Bribe,
                PlannerOpKind::Threaten,
                PlannerOpKind::DeclareSupport,
                PlannerOpKind::PressForceClaim,
            ]
        );
        assert_eq!(
            GoalKind::SupportCandidateForOffice {
                office: entity(40),
                candidate: entity(41),
            }
            .relevant_op_kinds(),
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::ConsultRecord,
                PlannerOpKind::DeclareSupport,
            ]
        );
    }

    #[test]
    fn political_prerequisite_places_include_office_register_when_vacancy_belief_unknown() {
        let actor = entity(1);
        let office = entity(40);
        let archive = entity(10);
        let jurisdiction = entity(11);
        let record = entity(12);
        let faction = entity(13);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, office, record]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(record, EntityKind::Record);
        view.effective_places.insert(actor, jurisdiction);
        view.effective_places.insert(office, jurisdiction);
        view.effective_places.insert(record, archive);
        view.entities_at.insert(jurisdiction, vec![actor, office]);
        view.entities_at.insert(archive, vec![record]);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.office_data_map
            .insert(office, vacant_office("Steward", jurisdiction, faction));
        view.record_data.insert(
            record,
            worldwake_core::RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: archive,
                issuer: actor,
                consultation_ticks: 4,
                max_entries_per_consult: 2,
                entries: vec![worldwake_core::InstitutionalRecordEntry {
                    entry_id: worldwake_core::RecordEntryId(0),
                    claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                        office,
                        holder: None,
                        effective_tick: Tick(3),
                    },
                    recorded_tick: Tick(3),
                    supersedes: None,
                }],
                next_entry_id: 1,
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, record]),
            &BTreeSet::from([archive, jurisdiction]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            GoalKind::ClaimOffice { office }.prerequisite_places(
                &state,
                &RecipeRegistry::new(),
                &execution_budget(&ProfileFixture::default())
            ),
            vec![archive]
        );
    }

    #[test]
    fn free_carry_capacity_goal_relevant_places_use_actor_place() {
        let (view, actor, place, waste_lot) = free_carry_capacity_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            GoalKind::FreeCarryCapacity.goal_relevant_places(&state, &RecipeRegistry::new()),
            vec![place]
        );
    }

    #[test]
    fn free_carry_capacity_contract_from_view_uses_carried_load_not_controlled_inventory_total() {
        let (mut view, actor, _place, _waste_lot) = free_carry_capacity_view();
        view.entity_loads.insert(entity(20), LoadUnits(6));
        view.commodity_quantities
            .insert((actor, CommodityKind::Waste), Quantity(18));

        let contract = super::free_carry_capacity_contract_from_view(&view, actor)
            .expect("contract should resolve");

        assert_eq!(contract.current_load, LoadUnits(6));
        assert!(!contract.is_actionable());
    }

    #[test]
    fn matches_binding_for_explore_location_ignores_hypothesis() {
        let target_place = entity(99);
        let food_goal = GoalKind::ExploreLocation {
            target_place,
            motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                HomeostaticNeedId::Hunger,
            ),
            hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Apple,
            },
        };
        let water_goal = GoalKind::ExploreLocation {
            target_place,
            motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                HomeostaticNeedId::Thirst,
            ),
            hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Water,
            },
        };

        assert!(food_goal.matches_binding(&[target_place], PlannerOpKind::Travel));
        assert!(water_goal.matches_binding(&[target_place], PlannerOpKind::Travel));
    }

    // ── matches_binding tests ──────────────────────────────────────────

    mod matches_binding_tests {
        use super::*;

        fn id(slot: u32) -> EntityId {
            entity_id(slot, 1)
        }

        // ── LootCorpse ────────────────────────────────────────────────

        #[test]
        fn loot_corpse_match() {
            let corpse = id(1);
            let goal = GoalKind::LootCorpse { corpse };
            assert!(goal.matches_binding(&[corpse], PlannerOpKind::Loot));
        }

        #[test]
        fn loot_corpse_mismatch() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(!goal.matches_binding(&[id(2)], PlannerOpKind::Loot));
        }

        #[test]
        fn auxiliary_bypass() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Travel));
        }

        #[test]
        fn empty_targets_bypass() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(goal.matches_binding(&[], PlannerOpKind::Loot));
        }

        #[test]
        fn steal_item_match() {
            let target_item = id(3);
            let goal = GoalKind::StealItem { target_item };
            assert!(goal.matches_binding(&[target_item], PlannerOpKind::Attack));
        }

        #[test]
        fn steal_item_mismatch() {
            let goal = GoalKind::StealItem { target_item: id(3) };
            assert!(!goal.matches_binding(&[id(4)], PlannerOpKind::Attack));
        }

        #[test]
        fn accuse_match() {
            let accused = id(5);
            let goal = GoalKind::Accuse {
                crime_register: id(4),
                accused,
                violation_id: ViolationId(5),
            };
            assert!(goal.matches_binding(&[accused], PlannerOpKind::Accuse));
        }

        #[test]
        fn accuse_mismatch() {
            let goal = GoalKind::Accuse {
                crime_register: id(4),
                accused: id(5),
                violation_id: ViolationId(6),
            };
            assert!(!goal.matches_binding(&[id(6)], PlannerOpKind::Accuse));
        }

        #[test]
        fn punish_accused_match() {
            let accused = id(7);
            let goal = GoalKind::PunishAccused {
                office: id(6),
                accused,
                accusation_entry: RecordEntryId(9),
                punishment: PunishmentKind::Exile {
                    from_faction: id(8),
                },
            };
            assert!(goal.matches_binding(&[accused], PlannerOpKind::Exile));
        }

        #[test]
        fn punish_accused_mismatch() {
            let goal = GoalKind::PunishAccused {
                office: id(6),
                accused: id(7),
                accusation_entry: RecordEntryId(10),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(2),
                },
            };
            assert!(!goal.matches_binding(&[id(9)], PlannerOpKind::Fine));
        }

        // ── Flexible goals ────────────────────────────────────────────

        #[test]
        fn flexible_goal_sleep() {
            let goal = GoalKind::Sleep;
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Attack));
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Loot));
            assert!(goal.matches_binding(&[], PlannerOpKind::Sleep));
        }

        #[test]
        fn flexible_goal_consume_owned() {
            let goal = GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water,
            };
            assert!(goal.matches_binding(&[id(5)], PlannerOpKind::Loot));
        }

        #[test]
        fn flexible_goal_reduce_danger() {
            let goal = GoalKind::ReduceDanger;
            assert!(goal.matches_binding(&[id(5)], PlannerOpKind::Attack));
        }

        #[test]
        fn flexible_goal_regroup_with_faction() {
            let goal = GoalKind::RegroupWithFaction { faction: id(5) };
            assert!(goal.matches_binding(&[id(6)], PlannerOpKind::Attack));
            assert!(goal.matches_binding(&[id(6)], PlannerOpKind::Travel));
        }

        // ── EngageHostile ─────────────────────────────────────────────

        #[test]
        fn engage_hostile_match() {
            let target = id(10);
            let goal = GoalKind::EngageHostile { target };
            assert!(goal.matches_binding(&[target], PlannerOpKind::Attack));
        }

        #[test]
        fn engage_hostile_mismatch() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            assert!(!goal.matches_binding(&[id(11)], PlannerOpKind::Attack));
        }

        #[test]
        fn raid_target_match() {
            let target = id(12);
            let goal = GoalKind::RaidTarget { target };
            assert!(goal.matches_binding(&[target], PlannerOpKind::Attack));
        }

        #[test]
        fn raid_target_mismatch() {
            let goal = GoalKind::RaidTarget { target: id(12) };
            assert!(!goal.matches_binding(&[id(13)], PlannerOpKind::Attack));
        }

        // ── TreatWounds ──────────────────────────────────────────────

        #[test]
        fn treat_wounds_match() {
            let patient = id(20);
            let goal = GoalKind::TreatWounds { patient };
            assert!(goal.matches_binding(&[patient], PlannerOpKind::Heal));
        }

        #[test]
        fn treat_wounds_mismatch() {
            let goal = GoalKind::TreatWounds { patient: id(20) };
            assert!(!goal.matches_binding(&[id(21)], PlannerOpKind::Heal));
        }

        // ── ShareBelief ───────────────────────────────────────────────

        #[test]
        fn share_belief_match() {
            let listener = id(30);
            let goal = GoalKind::ShareBelief {
                listener,
                topic: TellTopic::EntityBelief { subject: id(99) },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            };
            assert!(goal.matches_binding(&[listener], PlannerOpKind::Tell));
        }

        #[test]
        fn share_belief_mismatch() {
            let goal = GoalKind::ShareBelief {
                listener: id(30),
                topic: TellTopic::EntityBelief { subject: id(99) },
                communication_class: worldwake_core::CommunicationClass::Gossip,
            };
            assert!(!goal.matches_binding(&[id(31)], PlannerOpKind::Tell));
        }

        #[test]
        fn investigate_violation_matches_place_for_investigate_op() {
            let place = id(35);
            let goal = GoalKind::InvestigateViolation {
                violation_id: worldwake_core::ViolationId(1),
                place,
            };
            assert!(goal.matches_binding(&[place], PlannerOpKind::Investigate));
        }

        #[test]
        fn investigate_violation_rejects_other_place_for_investigate_op() {
            let goal = GoalKind::InvestigateViolation {
                violation_id: worldwake_core::ViolationId(2),
                place: id(35),
            };
            assert!(!goal.matches_binding(&[id(36)], PlannerOpKind::Investigate));
        }

        // ── MoveCargo ─────────────────────────────────────────────────

        #[test]
        fn move_cargo_destination_match() {
            let dest = id(40);
            let goal = GoalKind::MoveCargo {
                commodity: CommodityKind::Water,
                destination: dest,
            };
            assert!(goal.matches_binding(&[dest], PlannerOpKind::Loot));
        }

        #[test]
        fn move_cargo_destination_mismatch() {
            let goal = GoalKind::MoveCargo {
                commodity: CommodityKind::Water,
                destination: id(40),
            };
            assert!(!goal.matches_binding(&[id(41)], PlannerOpKind::Loot));
        }

        // ── BuryCorpse ───────────────────────────────────────────────

        #[test]
        fn bury_corpse_matches_corpse() {
            let corpse = id(50);
            let goal = GoalKind::BuryCorpse {
                corpse,
                burial_site: id(51),
            };
            assert!(goal.matches_binding(&[corpse], PlannerOpKind::Bury));
        }

        #[test]
        fn bury_corpse_matches_burial_site() {
            let burial_site = id(51);
            let goal = GoalKind::BuryCorpse {
                corpse: id(50),
                burial_site,
            };
            assert!(goal.matches_binding(&[burial_site], PlannerOpKind::Bury));
        }

        #[test]
        fn bury_corpse_mismatch() {
            let goal = GoalKind::BuryCorpse {
                corpse: id(50),
                burial_site: id(51),
            };
            assert!(!goal.matches_binding(&[id(52)], PlannerOpKind::Bury));
        }

        // ── DeclareSupport (always passes) ────────────────────────────

        #[test]
        fn claim_office_declare_support_passes() {
            let goal = GoalKind::ClaimOffice { office: id(60) };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::DeclareSupport));
        }

        #[test]
        fn support_candidate_declare_support_passes() {
            let goal = GoalKind::SupportCandidateForOffice {
                office: id(60),
                candidate: id(61),
            };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::DeclareSupport));
        }

        // ── All auxiliary ops bypass on exact-bound goal ──────────────

        #[test]
        fn all_auxiliary_ops_bypass() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            let unrelated = &[id(99)];
            let auxiliary_ops = [
                PlannerOpKind::Travel,
                PlannerOpKind::Trade,
                PlannerOpKind::Harvest,
                PlannerOpKind::Craft,
                PlannerOpKind::QueueForFacilityUse,
                PlannerOpKind::MoveCargo,
                PlannerOpKind::Consume,
                PlannerOpKind::Sleep,
                PlannerOpKind::Relieve,
                PlannerOpKind::Wash,
                PlannerOpKind::Defend,
                PlannerOpKind::Bribe,
                PlannerOpKind::Threaten,
            ];
            for op in auxiliary_ops {
                assert!(
                    goal.matches_binding(unrelated, op),
                    "auxiliary op {op:?} should bypass binding"
                );
            }
        }

        // ── Empty targets bypass on all terminal ops ──────────────────

        #[test]
        fn empty_targets_bypass_all_terminal_ops() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            let terminal_ops = [
                PlannerOpKind::Attack,
                PlannerOpKind::Loot,
                PlannerOpKind::Heal,
                PlannerOpKind::Tell,
                PlannerOpKind::DeclareSupport,
                PlannerOpKind::Bury,
            ];
            for op in terminal_ops {
                assert!(
                    goal.matches_binding(&[], op),
                    "empty targets should bypass terminal op {op:?}"
                );
            }
        }
    }

    #[test]
    fn claim_office_progress_barrier_still_active() {
        let office = entity(40);
        let goal = GoalKind::ClaimOffice { office };
        let step = PlannedStep {
            def_id: ActionDefId(77),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::DeclareSupport,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        assert!(
            goal.is_progress_barrier(&step),
            "ClaimOffice + DeclareSupport should still be a typed barrier"
        );
    }

    #[test]
    fn support_candidate_is_satisfied_unchanged_regression() {
        let actor = entity(1);
        let office = entity(40);
        let candidate = entity(41);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, candidate, office, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(candidate, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(candidate, town);
        view.effective_places.insert(office, town);
        view.entities_at
            .insert(town, vec![actor, candidate, office]);
        // Actor already declared support for candidate
        view.support_declaration_beliefs.insert(
            (office, actor),
            InstitutionalBeliefRead::Certain(Some(candidate)),
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, candidate]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SupportCandidateForOffice { office, candidate };
        assert!(
            goal.is_satisfied(&state),
            "SupportCandidateForOffice should still work when actor already declared"
        );

        // And not satisfied when no declaration
        let mut view2 = TestBeliefView::default();
        view2.alive.extend([actor, candidate, office, town]);
        view2.kinds.insert(actor, EntityKind::Agent);
        view2.kinds.insert(candidate, EntityKind::Agent);
        view2.kinds.insert(office, EntityKind::Office);
        view2.kinds.insert(town, EntityKind::Place);
        view2.effective_places.insert(actor, town);
        view2.effective_places.insert(candidate, town);
        view2.effective_places.insert(office, town);
        view2
            .entities_at
            .insert(town, vec![actor, candidate, office]);

        let snapshot2 = build_planning_snapshot(
            &view2,
            actor,
            &BTreeSet::from([office, candidate]),
            &BTreeSet::from([town]),
            1,
        );
        let state2 = PlanningState::new(&snapshot2);
        assert!(
            !goal.is_satisfied(&state2),
            "SupportCandidateForOffice should NOT be satisfied without declaration"
        );
    }

    #[test]
    fn free_carry_capacity_is_not_satisfied_above_disposal_threshold() {
        let (mut view, actor, place, waste_lot) = free_carry_capacity_view();
        view.disposal_profiles.insert(
            actor,
            DisposalProfile {
                capacity_strain_threshold: pm(800),
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(!GoalKind::FreeCarryCapacity.is_satisfied(&state));
    }

    #[test]
    fn free_carry_capacity_is_satisfied_below_disposal_threshold() {
        let (mut view, actor, place, waste_lot) = free_carry_capacity_view();
        view.commodity_quantities
            .insert((actor, CommodityKind::Waste), Quantity(7));
        view.commodity_quantities
            .insert((waste_lot, CommodityKind::Waste), Quantity(7));
        view.disposal_profiles.insert(
            actor,
            DisposalProfile {
                capacity_strain_threshold: pm(800),
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(GoalKind::FreeCarryCapacity.is_satisfied(&state));
    }

    #[test]
    fn free_carry_capacity_uses_default_threshold_without_profile() {
        let (view, actor, place, waste_lot) = free_carry_capacity_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let base_state = PlanningState::new(&snapshot);

        assert!(!GoalKind::FreeCarryCapacity.is_satisfied(&base_state));

        let progressed = base_state.move_lot_ref_to_ground(
            crate::PlanningEntityRef::Authoritative(waste_lot),
            place,
            CommodityKind::Waste,
            Quantity(9),
        );

        assert!(
            GoalKind::FreeCarryCapacity.is_satisfied(&progressed),
            "without an explicit DisposalProfile, FreeCarryCapacity should use the default threshold and still require disposal progress"
        );
    }

    #[test]
    fn free_carry_capacity_is_not_satisfied_after_partial_drop_still_at_threshold() {
        let (mut view, actor, place, waste_lot) = free_carry_capacity_view();
        view.disposal_profiles.insert(
            actor,
            DisposalProfile {
                capacity_strain_threshold: pm(800),
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let base_state = PlanningState::new(&snapshot);

        let progressed = base_state
            .with_commodity_quantity(actor, CommodityKind::Waste, Quantity(8))
            .with_commodity_quantity(waste_lot, CommodityKind::Waste, Quantity(8));

        assert!(
            !GoalKind::FreeCarryCapacity.is_satisfied(&progressed),
            "FreeCarryCapacity should remain unsatisfied after partial progress that still leaves the actor at or above the active threshold with lawful waste drop targets"
        );
    }

    // ── E16DPOLPLAN-006: Integration tests — planner finds Bribe/Threaten plans ──

    fn build_registry() -> (ActionDefRegistry, worldwake_sim::ActionHandlerRegistry) {
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();
        (registries.defs, registries.handlers)
    }

    fn epistemic_profile() -> EpistemicDispositionProfile {
        EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: Permille::new(400).unwrap(),
            witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
            ask_memory_retention_ticks: 10,
            witness_recency_preference: Permille::new(500).unwrap(),
        }
    }

    fn believed_entity_state_at(
        place: EntityId,
        observed_tick: Tick,
        resource_source: Option<ResourceSource>,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                observed_tick,
                worldwake_core::PerceptionSource::DirectObservation,
            )
        }
    }

    fn set_office_jurisdiction(
        view: &mut TestBeliefView,
        office: EntityId,
        jurisdiction: EntityId,
    ) {
        view.office_data_map.insert(
            office,
            OfficeData {
                title: String::new(),
                seat: jurisdiction,
                jurisdiction: BTreeSet::from([jurisdiction]),
                succession_law: SuccessionLaw::Support,
                eligibility_rules: vec![],
                succession_period_ticks: 5,
                vacancy_since: Some(Tick(0)),
            },
        );
    }

    fn claim_office_goal(office: EntityId) -> GoalOffer {
        GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::ClaimOffice { office }),
            evidence_entities: BTreeSet::from([office]),
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

    #[test]
    fn search_restock_goal_returns_travel_barrier_for_remote_stale_source() {
        let actor = entity(1);
        let subject_entity = entity(2);
        let town = entity(10);
        let remote = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, subject_entity, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(subject_entity, EntityKind::Facility);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(subject_entity, remote);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(remote, vec![subject_entity]);
        view.adjacent
            .insert(town, vec![(remote, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(remote, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                subject_entity,
                believed_entity_state_at(
                    remote,
                    Tick(0),
                    Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(4),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    }),
                ),
            )],
        );

        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(remote),
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::from([subject_entity]),
            evidence_places: BTreeSet::from([remote]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([subject_entity]),
            &BTreeSet::from([town, remote]),
            2,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(5),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a travel barrier plan");

        assert!(plan.terminal_kind.is_barrier());
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(
            plan.steps[0].targets,
            vec![crate::PlanningEntityRef::Authoritative(remote)]
        );
        assert_eq!(plan.steps[0].payload_override, None);
    }

    #[test]
    fn search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload() {
        let actor = entity(1);
        let witness = entity(2);
        let subject_entity = entity(3);
        let town = entity(10);
        let remote = entity(11);

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, witness, subject_entity, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(witness, EntityKind::Agent);
        view.kinds.insert(subject_entity, EntityKind::Facility);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.current_tick = Tick(50);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(witness, town);
        view.effective_places.insert(subject_entity, remote);
        view.entities_at.insert(town, vec![actor, witness]);
        view.entities_at.insert(remote, vec![subject_entity]);
        view.epistemic_profiles.insert(actor, epistemic_profile());
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                subject_entity,
                believed_entity_state_at(
                    remote,
                    Tick(0),
                    Some(ResourceSource {
                        commodity: CommodityKind::Bread,
                        available_quantity: Quantity(4),
                        max_quantity: Quantity(4),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    }),
                ),
            )],
        );

        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(remote),
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::from([subject_entity]),
            evidence_places: BTreeSet::from([remote]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([witness, subject_entity]),
            &BTreeSet::from([town, remote]),
            2,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(5),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a colocated ask_witness barrier plan");

        assert!(plan.terminal_kind.is_barrier());
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::AskWitness);
        assert_eq!(
            plan.steps[0].payload_override,
            Some(ActionPayload::AskWitness(AskWitnessPayload {
                target: witness,
                topic_entity: Some(subject_entity),
                topic_commodity: Some(CommodityKind::Bread),
            }))
        );
    }

    #[test]
    fn search_regroup_goal_uses_believed_rally_point_as_travel_destination() {
        let actor = entity(1);
        let faction = entity(30);
        let town = entity(10);
        let rally = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, rally]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(rally, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.adjacent
            .insert(town, vec![(rally, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(rally, vec![(town, NonZeroU32::new(2).unwrap())]);
        view.faction_rally_point_beliefs
            .insert(faction, InstitutionalBeliefRead::Certain(Some(rally)));
        view.known_institutional_beliefs.insert(
            actor,
            vec![BelievedInstitutionalClaim {
                claim: InstitutionalClaim::FactionRallyPoint {
                    faction,
                    rally_place: Some(rally),
                    effective_tick: Tick(4),
                },
                source: InstitutionalKnowledgeSource::DirectObservation,
                learned_tick: Tick(4),
                learned_at: Some(town),
            }],
        );

        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Place(rally),
            key: GoalKey::from(GoalKind::RegroupWithFaction { faction }),
            evidence_entities: BTreeSet::from([faction]),
            evidence_places: BTreeSet::from([rally]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([faction]),
            &BTreeSet::from([rally]),
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(5),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a regroup travel plan");

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(
            plan.steps[0].targets,
            vec![crate::PlanningEntityRef::Authoritative(rally)]
        );
    }

    #[test]
    fn search_raid_goal_uses_colocated_attack_affordance() {
        let actor = entity(1);
        let target = entity(2);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, town);
        view.entities_at.insert(town, vec![actor, target]);

        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(target),
            key: GoalKey::from(GoalKind::RaidTarget { target }),
            evidence_entities: BTreeSet::from([target]),
            evidence_places: BTreeSet::from([town]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([target]),
            &BTreeSet::from([town]),
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(5),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a colocated raid attack plan");

        assert_eq!(
            plan.terminal_kind,
            crate::PlanTerminalKind::CombatCommitment
        );
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Attack);
        assert_eq!(
            plan.steps[0].targets,
            vec![crate::PlanningEntityRef::Authoritative(target)]
        );
    }

    #[test]
    fn raid_goal_is_not_already_satisfied_for_colocated_non_hostile_prey() {
        let actor = entity(1);
        let target = entity(2);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, town);
        view.entities_at.insert(town, vec![actor, target]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([target]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::RaidTarget { target }.is_satisfied(&state),
            "co-located live prey should remain a live raid opportunity even without a hostility relation"
        );
    }

    #[test]
    fn search_remote_raid_goal_does_not_fabricate_attack_commitment() {
        let actor = entity(1);
        let target = entity(2);
        let town = entity(10);
        let road = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target, town, road]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(road, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, road);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(road, vec![target]);
        view.hostiles.insert(actor, vec![target]);

        let goal = GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(target),
            key: GoalKey::from(GoalKind::RaidTarget { target }),
            evidence_entities: BTreeSet::from([target]),
            evidence_places: BTreeSet::from([road]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([target]),
            &BTreeSet::from([road]),
            1,
        );
        let result = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(5),
            None,
            None,
        );

        match result {
            crate::PlanSearchResult::Found(plan) => {
                assert!(
                    plan.steps.is_empty(),
                    "remote raid should not fabricate an attack step: {plan:?}"
                );
                assert_eq!(plan.terminal_kind, crate::PlanTerminalKind::GoalSatisfied);
            }
            crate::PlanSearchResult::FrontierExhausted { .. }
            | crate::PlanSearchResult::BudgetExhausted { .. }
            | crate::PlanSearchResult::Unsupported => {}
        }
    }

    /// Test 1: Planner selects Bribe plan when competitor has existing support.
    ///
    /// Setup: actor at jurisdiction with coins, bribable target, vacant office.
    /// A competitor (rival) is at a DIFFERENT place but has self-declared support,
    /// so `DeclareSupport` alone would produce a tie (`typed barrier`). The rival
    /// cannot be bribed directly (not co-located). `Bribe(target)` + `DeclareSupport`
    /// gives a winning coalition (`GoalSatisfied`).
    #[test]
    fn planner_selects_bribe_plan() {
        let actor = entity(1);
        let target = entity(2); // bribable agent at same place
        let rival = entity(3); // competitor NOT at actor's place
        let office = entity(40);
        let town = entity(10);
        let remote = entity(11); // rival's location

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, target, rival, office, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(rival, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, town);
        view.effective_places.insert(rival, remote);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, target, office]);
        view.entities_at.insert(remote, vec![rival]);

        // Actor has coins for bribing
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(5));

        // Rival has self-declared support — creates competition
        view.support_declaration_beliefs.insert(
            (office, rival),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        // Target has high courage — Threaten won't work (default attack_skill=620)
        view.courage_values
            .insert(target, Permille::new(900).unwrap());

        set_office_jurisdiction(&mut view, office, town);
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, target, rival]),
            &BTreeSet::from([town]),
            1,
        );
        let goal = claim_office_goal(office);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(0),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a plan with Bribe");

        let op_kinds: Vec<_> = plan.steps.iter().map(|s| s.op_kind).collect();
        assert!(
            op_kinds.contains(&PlannerOpKind::Bribe),
            "plan should contain Bribe, got: {op_kinds:?}"
        );
        assert!(
            op_kinds.contains(&PlannerOpKind::DeclareSupport),
            "plan should contain DeclareSupport, got: {op_kinds:?}"
        );
    }

    /// Test 2: Planner selects `Threaten` plan when `attack_skill` > target courage.
    ///
    /// Setup: actor at jurisdiction with high `attack_skill`, low-courage target.
    /// A competitor (rival) is at a DIFFERENT place but has self-declared support,
    /// motivating the planner to select `Threaten` rather than relying on
    /// `DeclareSupport` alone.
    #[test]
    fn planner_selects_threaten_plan() {
        let actor = entity(1);
        let target = entity(2); // low-courage agent at same place
        let rival = entity(3); // competitor NOT at actor's place
        let office = entity(40);
        let town = entity(10);
        let remote = entity(11);

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, target, rival, office, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(rival, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, town);
        view.effective_places.insert(rival, remote);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, target, office]);
        view.entities_at.insert(remote, vec![rival]);

        // Actor has high attack_skill (default CombatProfile has attack_skill=620)
        // Target has low courage
        view.courage_values
            .insert(target, Permille::new(100).unwrap());

        // Rival has self-declared support — creates competition
        view.support_declaration_beliefs.insert(
            (office, rival),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        set_office_jurisdiction(&mut view, office, town);
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, target, rival]),
            &BTreeSet::from([town]),
            1,
        );
        let goal = claim_office_goal(office);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(0),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a plan with Threaten");

        let op_kinds: Vec<_> = plan.steps.iter().map(|s| s.op_kind).collect();
        assert!(
            op_kinds.contains(&PlannerOpKind::Threaten),
            "plan should contain Threaten, got: {op_kinds:?}"
        );
    }

    /// Test 3: Planner selects Travel + Bribe when actor is NOT at jurisdiction.
    ///
    /// Setup: actor at a remote place, has coins. Target and rival at the
    /// jurisdiction. Rival has self-declared support. Plan should start with
    /// `Travel` then include `Bribe` + `DeclareSupport`.
    #[test]
    fn planner_selects_travel_then_bribe() {
        let actor = entity(1);
        let target = entity(2); // bribable agent at jurisdiction
        let rival = entity(3); // competitor with existing support
        let office = entity(40);
        let town = entity(10); // jurisdiction
        let remote = entity(11); // actor starts here

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, target, rival, office, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(rival, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, remote);
        view.effective_places.insert(target, town);
        view.effective_places.insert(rival, town);
        view.effective_places.insert(office, town);
        view.entities_at.insert(remote, vec![actor]);
        view.entities_at.insert(town, vec![target, rival, office]);

        // Travel edge between remote and town
        view.adjacent
            .insert(remote, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(town, vec![(remote, NonZeroU32::new(3).unwrap())]);

        // Actor has coins for bribing
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(5));

        // Target has courage
        view.courage_values
            .insert(target, Permille::new(500).unwrap());

        // Rival has self-declared support — creates competition
        view.support_declaration_beliefs.insert(
            (office, rival),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        set_office_jurisdiction(&mut view, office, town);
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, target, rival]),
            &BTreeSet::from([town, remote]),
            2,
        );
        let goal = claim_office_goal(office);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(0),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a Travel+Bribe plan");

        let op_kinds: Vec<_> = plan.steps.iter().map(|s| s.op_kind).collect();
        assert!(
            op_kinds.contains(&PlannerOpKind::Travel),
            "plan should contain Travel, got: {op_kinds:?}"
        );
        assert!(
            op_kinds.contains(&PlannerOpKind::Bribe),
            "plan should contain Bribe after Travel, got: {op_kinds:?}"
        );

        // Travel must come before Bribe
        let travel_pos = op_kinds
            .iter()
            .position(|op| *op == PlannerOpKind::Travel)
            .unwrap();
        let bribe_pos = op_kinds
            .iter()
            .position(|op| *op == PlannerOpKind::Bribe)
            .unwrap();
        assert!(
            travel_pos < bribe_pos,
            "Travel (pos {travel_pos}) should come before Bribe (pos {bribe_pos})"
        );
    }

    /// Test 4: Planner rejects `Threaten` when target courage exceeds `attack_skill`.
    ///
    /// Setup: actor at jurisdiction, target with high courage (exceeds actor's
    /// `attack_skill`). Rival is at a DIFFERENT place but has self-declared support.
    /// `Threaten` would fail, so the planner falls back to `Bribe` (actor has coins)
    /// + `DeclareSupport`. Plan must NOT contain `Threaten`.
    #[test]
    fn planner_rejects_threaten_against_high_courage() {
        let actor = entity(1);
        let target = entity(2); // high-courage agent at same place
        let rival = entity(3); // competitor NOT at actor's place
        let office = entity(40);
        let town = entity(10);
        let remote = entity(11);

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, target, rival, office, town, remote]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Agent);
        view.kinds.insert(rival, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(remote, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(target, town);
        view.effective_places.insert(rival, remote);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, target, office]);
        view.entities_at.insert(remote, vec![rival]);

        // Target has very high courage — exceeds actor's default attack_skill (620)
        view.courage_values
            .insert(target, Permille::new(900).unwrap());

        // Actor has coins so planner can fall back to Bribe
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(5));

        // Rival has self-declared support — creates competition
        view.support_declaration_beliefs.insert(
            (office, rival),
            InstitutionalBeliefRead::Certain(Some(rival)),
        );

        set_office_jurisdiction(&mut view, office, town);
        view.office_holder_beliefs
            .insert(office, InstitutionalBeliefRead::Certain(None));

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, target, rival]),
            &BTreeSet::from([town]),
            1,
        );
        let goal = claim_office_goal(office);
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &RecipeRegistry::new(),
            &BlockerMemory::default(),
            Tick(0),
            None,
            None,
        )
        .into_plan()
        .expect("planner should find a plan without Threaten");

        let op_kinds: Vec<_> = plan.steps.iter().map(|s| s.op_kind).collect();
        assert!(
            !op_kinds.contains(&PlannerOpKind::Threaten),
            "plan should NOT contain Threaten against high-courage target, got: {op_kinds:?}"
        );
    }

    #[test]
    fn sell_commodity_staff_market_is_progress_barrier() {
        let goal = GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        };
        let step = PlannedStep {
            def_id: ActionDefId(200),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::StaffMarket,
            estimated_ticks: 5,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        assert!(
            goal.is_progress_barrier(&step),
            "SellCommodity + StaffMarket should be a typed barrier"
        );
    }

    #[test]
    fn sell_commodity_travel_is_not_progress_barrier() {
        let goal = GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        };
        let step = PlannedStep {
            def_id: ActionDefId(201),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        assert!(
            !goal.is_progress_barrier(&step),
            "SellCommodity + Travel should NOT be a typed barrier"
        );
    }

    #[test]
    fn sell_commodity_satisfied_when_at_home_facility_with_listed_lot() {
        let actor = entity_id(1, 0);
        let market = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let display_container = entity_id(5, 0);
        let bread_lot = entity_id(4, 0);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, facility, display_container, bread_lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.kinds.insert(display_container, EntityKind::Container);
        view.kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, market);
        view.effective_places.insert(facility, market);
        view.effective_places.insert(display_container, market);
        view.effective_places.insert(bread_lot, market);
        view.entities_at
            .insert(market, vec![actor, facility, display_container, bread_lot]);
        view.direct_containers.insert(bread_lot, display_container);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(3));
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );
        view.stock_storage_policies.insert(
            facility,
            worldwake_core::StockStoragePolicy {
                stock_container: entity_id(6, 0),
                display_container: Some(display_container),
            },
        );
        view.listed_lots
            .insert((market, CommodityKind::Bread), vec![bread_lot]);
        view.lot_sellers.insert(bread_lot, actor);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread_lot]),
            &BTreeSet::from([market]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn sell_commodity_not_satisfied_when_not_at_home_facility() {
        let actor = entity_id(1, 0);
        let market = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let other = entity_id(4, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, other);
        view.effective_places.insert(facility, market);
        view.entities_at.insert(market, vec![facility]);
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([market, other]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn sell_commodity_not_satisfied_when_no_listed_lot() {
        let actor = entity_id(1, 0);
        let market = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, market);
        view.effective_places.insert(facility, market);
        view.entities_at.insert(market, vec![actor, facility]);
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([market]), 1);
        let state = PlanningState::new(&snapshot);

        assert!(
            !GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            }
            .is_satisfied(&state)
        );
    }

    #[test]
    fn fulfill_bounty_is_satisfied_when_believed_bounty_is_non_active() {
        let actor = entity_id(1, 0);
        let bounty = entity_id(2, 0);
        let issuer = entity_id(3, 0);
        let claim_place = entity_id(4, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bounty, EntityKind::SocialArtifact);
        view.effective_places.insert(actor, claim_place);
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                bounty,
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
                    believed_artifact: Some(believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::EliminateEntity {
                            target: entity_id(5, 0),
                        },
                        ArtifactActionability::Closed {
                            closed_at: Tick(1),
                            cause: CloseCause::BountyFulfilled,
                        },
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bounty]),
            &BTreeSet::from([claim_place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(GoalKind::FulfillBounty { bounty }.is_satisfied(&state));
    }

    #[test]
    fn fulfill_bounty_elimination_availability_switches_from_attack_to_claim_after_target_death() {
        let actor = entity_id(1, 0);
        let bounty = entity_id(2, 0);
        let issuer = entity_id(3, 0);
        let claim_place = entity_id(4, 0);
        let target = entity_id(5, 0);
        let goal = GoalKind::FulfillBounty { bounty };

        let mut live_view = TestBeliefView::default();
        live_view.alive.insert(actor);
        live_view.alive.insert(target);
        live_view.kinds.insert(actor, EntityKind::Agent);
        live_view.kinds.insert(target, EntityKind::Agent);
        live_view.kinds.insert(bounty, EntityKind::SocialArtifact);
        live_view.effective_places.insert(actor, claim_place);
        live_view.effective_places.insert(target, claim_place);
        live_view.known_entity_beliefs.insert(
            actor,
            vec![(
                bounty,
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
                    believed_artifact: Some(believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::EliminateEntity { target },
                        ArtifactActionability::Actionable,
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );
        let live_snapshot = build_planning_snapshot(
            &live_view,
            actor,
            &BTreeSet::from([bounty, target]),
            &BTreeSet::from([claim_place]),
            1,
        );
        let live_state = PlanningState::new(&live_snapshot);
        assert!(goal.candidate_is_available(&live_state, PlannerOpKind::Attack));
        assert!(!goal.candidate_is_available(&live_state, PlannerOpKind::ClaimBounty));

        let mut dead_view = live_view;
        dead_view.alive.remove(&target);
        let dead_snapshot = build_planning_snapshot(
            &dead_view,
            actor,
            &BTreeSet::from([bounty, target]),
            &BTreeSet::from([claim_place]),
            1,
        );
        let dead_state = PlanningState::new(&dead_snapshot);
        assert!(!goal.candidate_is_available(&dead_state, PlannerOpKind::Attack));
        assert!(goal.candidate_is_available(&dead_state, PlannerOpKind::ClaimBounty));
    }

    #[test]
    fn fulfill_bounty_relevant_ops_include_delivery_cargo_surfaces() {
        let goal = GoalKind::FulfillBounty {
            bounty: entity_id(2, 0),
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::MoveCargo));
        assert!(
            goal.relevant_op_kinds()
                .contains(&PlannerOpKind::StockManagement)
        );
        assert!(
            goal.relevant_op_kinds()
                .contains(&PlannerOpKind::ClaimBounty)
        );
    }

    #[test]
    fn free_carry_capacity_candidate_is_available_only_for_drop_item_with_waste() {
        let (view, actor, place, waste_lot) = free_carry_capacity_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([waste_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(
            GoalKind::FreeCarryCapacity.candidate_is_available(&state, PlannerOpKind::DropItem)
        );
        assert!(!GoalKind::FreeCarryCapacity.candidate_is_available(&state, PlannerOpKind::Trade));

        let clean_lot = entity(21);
        let mut clean_view = TestBeliefView::default();
        clean_view.alive.extend([actor, place, clean_lot]);
        clean_view.kinds.insert(actor, EntityKind::Agent);
        clean_view.kinds.insert(place, EntityKind::Place);
        clean_view.kinds.insert(clean_lot, EntityKind::ItemLot);
        clean_view.effective_places.insert(actor, place);
        clean_view.effective_places.insert(clean_lot, place);
        clean_view.entities_at.insert(place, vec![actor, clean_lot]);
        clean_view.direct_possessions.insert(actor, vec![clean_lot]);
        clean_view.direct_possessors.insert(clean_lot, actor);
        clean_view
            .lot_commodities
            .insert(clean_lot, CommodityKind::Bread);
        clean_view
            .commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        clean_view
            .commodity_quantities
            .insert((clean_lot, CommodityKind::Bread), Quantity(1));
        clean_view.carry_capacities.insert(actor, LoadUnits(10));
        clean_view.entity_loads.insert(actor, LoadUnits(0));

        let clean_snapshot = build_planning_snapshot(
            &clean_view,
            actor,
            &BTreeSet::from([clean_lot]),
            &BTreeSet::from([place]),
            1,
        );
        let clean_state = PlanningState::new(&clean_snapshot);

        assert!(
            !GoalKind::FreeCarryCapacity
                .candidate_is_available(&clean_state, PlannerOpKind::DropItem)
        );
    }

    #[test]
    fn accuse_candidate_is_available_only_at_crime_register_home_place() {
        let actor = entity(1);
        let accused = entity(2);
        let register = entity(3);
        let square = entity(4);
        let hall = entity(5);
        let accusation_entry = worldwake_core::RecordEntryId(1);
        let claim = worldwake_core::InstitutionalClaim::Accusation {
            accuser: actor,
            accused,
            violation_id: worldwake_core::ViolationId(7),
            theft: worldwake_core::TheftFacts {
                missing_entity: entity(7),
                expected_place: square,
                commodity: CommodityKind::Bread,
                quantity: Quantity(1),
            },
            effective_tick: Tick(1),
        };

        let mut remote_view = TestBeliefView::default();
        remote_view
            .alive
            .extend([actor, accused, register, square, hall]);
        remote_view.kinds.insert(actor, EntityKind::Agent);
        remote_view.kinds.insert(accused, EntityKind::Agent);
        remote_view.kinds.insert(register, EntityKind::Record);
        remote_view.kinds.insert(square, EntityKind::Place);
        remote_view.kinds.insert(hall, EntityKind::Place);
        remote_view.effective_places.insert(actor, square);
        remote_view.effective_places.insert(accused, square);
        remote_view.effective_places.insert(register, hall);
        remote_view.entities_at.insert(square, vec![actor, accused]);
        remote_view.entities_at.insert(hall, vec![register]);
        remote_view.record_data.insert(
            register,
            worldwake_core::RecordData {
                record_kind: worldwake_core::RecordKind::CrimeRegister,
                home_place: hall,
                issuer: actor,
                consultation_ticks: 1,
                max_entries_per_consult: 4,
                entries: vec![worldwake_core::InstitutionalRecordEntry {
                    entry_id: accusation_entry,
                    claim,
                    recorded_tick: Tick(1),
                    supersedes: None,
                }],
                next_entry_id: 2,
            },
        );

        let goal = GoalKind::Accuse {
            crime_register: register,
            accused,
            violation_id: worldwake_core::ViolationId(7),
        };

        let remote_snapshot = build_planning_snapshot(
            &remote_view,
            actor,
            &BTreeSet::from([accused, register]),
            &BTreeSet::from([square, hall]),
            1,
        );
        let remote_state = PlanningState::new(&remote_snapshot);
        assert!(
            !goal.candidate_is_available(&remote_state, PlannerOpKind::Accuse),
            "accuse should not be locally available away from the crime register"
        );

        remote_view.effective_places.insert(actor, hall);
        remote_view.entities_at.insert(hall, vec![actor, register]);
        let local_snapshot = build_planning_snapshot(
            &remote_view,
            actor,
            &BTreeSet::from([accused, register]),
            &BTreeSet::from([square, hall]),
            1,
        );
        let local_state = PlanningState::new(&local_snapshot);
        assert!(goal.candidate_is_available(&local_state, PlannerOpKind::Accuse));
    }

    #[test]
    fn fulfill_bounty_delivery_relevant_place_is_destination_until_delivered() {
        let actor = entity_id(1, 0);
        let bounty = entity_id(2, 0);
        let issuer = entity_id(3, 0);
        let destination = entity_id(4, 0);
        let claim_place = entity_id(5, 0);
        let bread_lot = entity_id(6, 0);
        let source = entity_id(7, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bounty, EntityKind::SocialArtifact);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(claim_place, EntityKind::Place);
        view.kinds.insert(source, EntityKind::Place);
        view.kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, source);
        view.effective_places.insert(bread_lot, source);
        view.entities_at.insert(source, vec![actor, bread_lot]);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((actor, bread_lot));
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                bounty,
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
                    believed_artifact: Some(believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        ArtifactActionability::Actionable,
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bounty, bread_lot]),
            &BTreeSet::from([source, destination, claim_place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            GoalKind::FulfillBounty { bounty }.goal_relevant_places(&state, &RecipeRegistry::new()),
            vec![destination]
        );
    }

    #[test]
    fn fulfill_bounty_delivery_relevant_place_becomes_claim_place_once_delivered() {
        let actor = entity_id(1, 0);
        let bounty = entity_id(2, 0);
        let issuer = entity_id(3, 0);
        let destination = entity_id(4, 0);
        let claim_place = entity_id(5, 0);
        let bread_lot = entity_id(6, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bounty, EntityKind::SocialArtifact);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(claim_place, EntityKind::Place);
        view.kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(bread_lot, destination);
        view.entities_at.insert(destination, vec![actor, bread_lot]);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((actor, bread_lot));
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                bounty,
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
                    believed_artifact: Some(believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        ArtifactActionability::Actionable,
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bounty, bread_lot]),
            &BTreeSet::from([destination, claim_place]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert_eq!(
            GoalKind::FulfillBounty { bounty }.goal_relevant_places(&state, &RecipeRegistry::new()),
            vec![claim_place]
        );
    }

    #[test]
    fn fulfill_bounty_delivery_builds_pick_up_payload_from_delivery_gap() {
        let actor = entity_id(1, 0);
        let bounty = entity_id(2, 0);
        let issuer = entity_id(3, 0);
        let destination = entity_id(4, 0);
        let claim_place = entity_id(5, 0);
        let source = entity_id(6, 0);
        let bread_lot = entity_id(7, 0);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, bounty, destination, claim_place, source, bread_lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bounty, EntityKind::SocialArtifact);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(claim_place, EntityKind::Place);
        view.kinds.insert(source, EntityKind::Place);
        view.kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, source);
        view.effective_places.insert(bread_lot, source);
        view.entities_at.insert(source, vec![actor, bread_lot]);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread_lot, CommodityKind::Bread), Quantity(5));
        view.carry_capacities.insert(actor, LoadUnits(2));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread_lot, LoadUnits(5));
        view.controllable.insert((actor, bread_lot));
        view.known_entity_beliefs.insert(
            actor,
            vec![(
                bounty,
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
                    believed_artifact: Some(believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        ArtifactActionability::Actionable,
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        worldwake_core::PerceptionSource::DirectObservation,
                    )
                },
            )],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bounty, bread_lot]),
            &BTreeSet::from([source, destination, claim_place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::FulfillBounty { bounty };
        let def = ActionDef {
            id: ActionDefId(9),
            name: "pick_up".to_string(),
            domain: ActionDomain::Transport,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[bread_lot], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(2),
            }))
        );
    }

    #[test]
    fn sell_commodity_relevant_places_returns_home_facility_place() {
        let actor = entity_id(1, 0);
        let market = entity_id(2, 0);
        let facility = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, market);
        view.effective_places.insert(facility, market);
        view.entities_at.insert(market, vec![actor, facility]);
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        );

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([market]), 1);
        let state = PlanningState::new(&snapshot);

        let places = GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        }
        .goal_relevant_places(&state, &RecipeRegistry::new());
        assert_eq!(places, vec![market]);
    }

    #[test]
    fn sell_commodity_relevant_places_empty_without_profile() {
        let actor = entity_id(1, 0);
        let market = entity_id(2, 0);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, market);

        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([market]), 1);
        let state = PlanningState::new(&snapshot);

        let places = GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        }
        .goal_relevant_places(&state, &RecipeRegistry::new());
        assert!(places.is_empty());
    }

    #[test]
    fn post_bounty_builds_payload_override_from_goal_terms() {
        let actor = entity(1);
        let posting_place = entity(10);
        let target = entity(20);
        let authority = entity(30);
        let snapshot = build_planning_snapshot(
            &TestBeliefView::default(),
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([posting_place]),
            0,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::PostBounty {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: Some(authority),
                expires_at: Some(Tick(12)),
                jurisdiction: Some(posting_place),
            },
            terms: BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: ProofRequirement::SelfReport,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(7),
                reward_source: RewardSource::PersonalFunds { issuer: actor },
                claim_place: posting_place,
            },
        };
        let def = ActionDef {
            id: ActionDefId(901),
            name: "post_bounty".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::NonInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::PostBounty,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[], &def, &semantics)
            .unwrap()
            .unwrap();
        assert_eq!(
            payload.as_post_bounty(),
            Some(&worldwake_sim::PostBountyActionPayload {
                posting_place,
                issuing_authority: Some(authority),
                expires_at: Some(Tick(12)),
                jurisdiction: Some(posting_place),
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: ProofRequirement::SelfReport,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(7),
                reward_source: RewardSource::PersonalFunds { issuer: actor },
                claim_place: posting_place,
            })
        );
    }

    #[test]
    fn post_notice_builds_payload_override_and_is_progress_barrier() {
        let actor = entity(1);
        let posting_place = entity(10);
        let authority = entity(30);
        let snapshot = build_planning_snapshot(
            &TestBeliefView::default(),
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([posting_place]),
            0,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: Some(authority),
                expires_at: Some(Tick(24)),
                jurisdiction: Some(posting_place),
            },
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        };
        let def = ActionDef {
            id: ActionDefId(902),
            name: "post_notice".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::NonInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::PostNotice,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        };

        let payload = goal
            .build_payload_override(None, &state, &[], &def, &semantics)
            .unwrap()
            .unwrap();
        assert_eq!(
            payload.as_post_notice(),
            Some(&worldwake_sim::PostNoticeActionPayload {
                posting_place,
                issuing_authority: Some(authority),
                expires_at: Some(Tick(24)),
                jurisdiction: Some(posting_place),
                topic: NoticeTopic::ThreatWarning {
                    place: posting_place,
                },
            })
        );
        assert!(goal.is_progress_barrier(&PlannedStep {
            def_id: def.id,
            targets: Vec::new(),
            target_place: None,
            payload_override: Some(payload),
            op_kind: PlannerOpKind::PostNotice,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }));
    }
}
