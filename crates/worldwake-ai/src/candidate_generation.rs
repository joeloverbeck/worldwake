use crate::{
    decision_trace::{
        PoliticalCandidateOmission, PoliticalCandidateOmissionReason, PoliticalGoalFamily,
    },
    derive_danger_pressure,
    enterprise::{analyze_candidate_enterprise, restock_gap_at_destination, EnterpriseSignals},
    GroundedGoal,
};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque};
use worldwake_core::{
    load_per_unit, BlockedIntentMemory, CommodityKind, CommodityPurpose, DriveThresholds,
    EligibilityRule, EntityId, EntityKind, GoalKey, GoalKind, HomeostaticNeeds, OfficeData,
    PerceptionSource, Quantity, Tick,
};
use worldwake_sim::{relayable_social_subjects, GoalBeliefView, RecipeDefinition, RecipeRegistry};

#[derive(Default)]
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

struct GenerationContext<'a> {
    view: &'a dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    travel_horizon: u8,
    enterprise: EnterpriseSignals,
    blocked: &'a BlockedIntentMemory,
    recipes: &'a RecipeRegistry,
    current_tick: Tick,
}

#[derive(Default)]
pub(crate) struct CandidateGenerationDiagnostics {
    pub omitted_political: Vec<PoliticalCandidateOmission>,
}

pub(crate) struct CandidateGenerationResult {
    pub candidates: Vec<GroundedGoal>,
    pub diagnostics: CandidateGenerationDiagnostics,
}

#[must_use]
pub fn generate_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal> {
    generate_candidates_with_travel_horizon(view, agent, blocked, recipes, current_tick, 6)
        .candidates
}

#[must_use]
pub(crate) fn generate_candidates_with_travel_horizon(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,
) -> CandidateGenerationResult {
    if view.is_dead(agent) || !view.is_alive(agent) {
        return CandidateGenerationResult {
            candidates: Vec::new(),
            diagnostics: CandidateGenerationDiagnostics::default(),
        };
    }

    let mut candidates = BTreeMap::new();
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
        recipes,
        current_tick,
    };

    emit_need_candidates(&mut candidates, &ctx, needs, thresholds);
    emit_production_candidates(&mut candidates, &ctx, needs, thresholds);
    emit_enterprise_candidates(&mut candidates, &ctx);
    emit_combat_candidates(&mut candidates, &ctx);
    emit_social_candidates(&mut candidates, &ctx);
    emit_political_candidates(&mut candidates, &mut diagnostics, &ctx);

    CandidateGenerationResult {
        candidates: candidates.into_values().collect(),
        diagnostics,
    }
}

fn emit_need_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
) {
    let (Some(needs), Some(thresholds)) = (needs, thresholds) else {
        return;
    };

    emit_self_consume_candidates(candidates, ctx, needs, thresholds);
    emit_sleep_goal(candidates, ctx, needs, thresholds);
    emit_relieve_goal(candidates, ctx, needs, thresholds);
    emit_wash_goal(candidates, ctx, needs, thresholds);
}

fn emit_production_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
) {
    emit_produce_goals(candidates, ctx, needs, thresholds);
}

fn emit_enterprise_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    emit_restock_goals(candidates, ctx);
    emit_move_cargo_goals(candidates, ctx);
}

fn emit_combat_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    emit_engage_hostile_goals(candidates, ctx);
    emit_reduce_danger_goal(candidates, ctx);
    emit_care_goals(candidates, ctx);
    emit_loot_goals(candidates, ctx);
    emit_bury_goals(candidates, ctx);
}

fn emit_social_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    let Some(place) = ctx.place else {
        return;
    };
    let Some(profile) = ctx.view.tell_profile(ctx.agent) else {
        return;
    };
    let known_beliefs = ctx.view.known_entity_beliefs(ctx.agent);

    let subjects = relayable_social_subjects(
        known_beliefs.clone(),
        profile.max_relay_chain_len,
        profile.max_tell_candidates,
    );
    if subjects.is_empty() {
        return;
    }

    for listener in social_listeners_at(ctx.view, ctx.agent, place) {
        for subject in subjects.iter().copied() {
            if known_beliefs.iter().any(|(known_subject, belief)| {
                *known_subject == subject && belief.last_known_place == Some(place)
            }) {
                continue;
            }
            let mut evidence = Evidence::with_entity(listener);
            evidence.entities.insert(subject);
            evidence.places.insert(place);
            emit_candidate(
                candidates,
                GoalKind::ShareBelief { listener, subject },
                evidence,
                ctx.blocked,
                ctx.current_tick,
            );
        }
    }
}

fn emit_political_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let known_entities = ctx.view.known_entity_beliefs(ctx.agent);
    for (office, _) in known_entities {
        if ctx.view.entity_kind(office) != Some(EntityKind::Office) {
            continue;
        }
        let Some(office_data) = ctx.view.office_data(office) else {
            continue;
        };
        if office_data.succession_law != worldwake_core::SuccessionLaw::Support {
            record_office_wide_political_omission(
                diagnostics,
                office,
                PoliticalCandidateOmissionReason::ForceSuccessionLaw,
            );
            continue;
        }
        if !office_is_visibly_vacant(ctx.view, office, &office_data) {
            record_office_wide_political_omission(
                diagnostics,
                office,
                PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant,
            );
            continue;
        }

        emit_claim_office_candidate(candidates, diagnostics, ctx, office, &office_data);
        emit_support_candidate_goals(candidates, diagnostics, ctx, office, &office_data);
    }
}

fn emit_claim_office_candidate(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
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
    if ctx.view.office_holder(office) == Some(ctx.agent) {
        return;
    }
    if ctx.view.support_declaration(ctx.agent, office) == Some(ctx.agent) {
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

    let mut evidence = Evidence::with_entity(office);
    evidence.entities.insert(ctx.agent);
    evidence.places.insert(office_data.jurisdiction);
    emit_candidate(
        candidates,
        GoalKind::ClaimOffice { office },
        evidence,
        ctx.blocked,
        ctx.current_tick,
    );
}

fn emit_support_candidate_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    office: EntityId,
    office_data: &OfficeData,
) {
    let current_declaration = ctx.view.support_declaration(ctx.agent, office);
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
        if current_declaration == Some(candidate) {
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

        let mut evidence = Evidence::with_entity(office);
        evidence.entities.insert(candidate);
        evidence.places.insert(office_data.jurisdiction);
        emit_candidate(
            candidates,
            GoalKind::SupportCandidateForOffice { office, candidate },
            evidence,
            ctx.blocked,
            ctx.current_tick,
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

fn office_is_visibly_vacant(
    view: &dyn GoalBeliefView,
    office: EntityId,
    office_data: &OfficeData,
) -> bool {
    office_data.vacancy_since.is_some() && view.office_holder(office).is_none()
}

fn candidate_is_eligible(
    view: &dyn GoalBeliefView,
    office_data: &OfficeData,
    candidate: EntityId,
) -> bool {
    view.entity_kind(candidate) == Some(EntityKind::Agent)
        && view.is_alive(candidate)
        && office_data
            .eligibility_rules
            .iter()
            .all(|rule| matches!(rule, EligibilityRule::FactionMember(faction) if view.factions_of(candidate).contains(faction)))
}

fn emit_engage_hostile_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
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

    for target in local_hostility_targets(ctx.view, ctx.agent, ctx.place) {
        if current_attackers.contains(&target) {
            continue;
        }

        let mut evidence = Evidence::with_entity(target);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        emit_candidate(
            candidates,
            GoalKind::EngageHostile { target },
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_self_consume_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    emit_need_driven_candidates(
        candidates,
        ctx,
        needs.hunger,
        thresholds.hunger.low(),
        relieves_hunger,
    );
    emit_need_driven_candidates(
        candidates,
        ctx,
        needs.thirst,
        thresholds.thirst.low(),
        relieves_thirst,
    );
}

fn emit_need_driven_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
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

        if let Some(evidence) = acquisition_path_evidence(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) {
            emit_candidate(
                candidates,
                GoalKind::AcquireCommodity {
                    commodity,
                    purpose: CommodityPurpose::SelfConsume,
                },
                evidence,
                ctx.blocked,
                ctx.current_tick,
            );
        }
    }
}

fn emit_sleep_goal(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    if needs.fatigue >= thresholds.fatigue.low() {
        emit_candidate(
            candidates,
            GoalKind::Sleep,
            Evidence::with_entity(ctx.agent),
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_relieve_goal(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    needs: HomeostaticNeeds,
    thresholds: DriveThresholds,
) {
    if needs.bladder >= thresholds.bladder.low() {
        emit_candidate(
            candidates,
            GoalKind::Relieve,
            Evidence::with_entity(ctx.agent),
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_wash_goal(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
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
        emit_candidate(
            candidates,
            GoalKind::Wash,
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_reduce_danger_goal(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
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
        emit_candidate(
            candidates,
            GoalKind::ReduceDanger,
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_care_goals(candidates: &mut BTreeMap<GoalKey, GroundedGoal>, ctx: &GenerationContext<'_>) {
    // Self-care: emit if agent believes self wounded (no medicine gate).
    if ctx.view.has_wounds(ctx.agent) {
        let mut evidence = Evidence::with_entity(ctx.agent);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        emit_candidate(
            candidates,
            GoalKind::TreatWounds { patient: ctx.agent },
            evidence,
            ctx.blocked,
            ctx.current_tick,
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
        emit_candidate(
            candidates,
            GoalKind::TreatWounds { patient: entity },
            evidence,
            ctx.blocked,
            ctx.current_tick,
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
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
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

        if let Some(mut evidence) = recipe_path_evidence(ctx.view, ctx.agent, ctx.place, recipe) {
            if let Some(place) = ctx.place {
                evidence.places.insert(place);
            }
            emit_candidate(
                candidates,
                GoalKind::ProduceCommodity { recipe_id },
                evidence,
                ctx.blocked,
                ctx.current_tick,
            );
            continue;
        }

        emit_missing_recipe_input_goals(candidates, ctx, recipe_id, recipe);
    }
}

fn emit_restock_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    let Some(profile) = ctx.view.merchandise_profile(ctx.agent) else {
        return;
    };

    for commodity in profile.sale_kinds {
        if ctx.enterprise.restock_gap(commodity).is_none() {
            continue;
        }
        if let Some(evidence) = acquisition_path_evidence(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) {
            emit_candidate(
                candidates,
                GoalKind::RestockCommodity { commodity },
                evidence,
                ctx.blocked,
                ctx.current_tick,
            );
        }
    }
}

fn emit_move_cargo_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
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
        evidence.entities.extend(local_lots);
        emit_candidate(
            candidates,
            GoalKind::MoveCargo {
                commodity,
                destination,
            },
            evidence,
            ctx.blocked,
            ctx.current_tick,
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

fn emit_loot_goals(candidates: &mut BTreeMap<GoalKey, GroundedGoal>, ctx: &GenerationContext<'_>) {
    let Some(place) = ctx.place else {
        return;
    };

    for corpse in ctx.view.corpse_entities_at(place) {
        if !corpse_has_known_loot(ctx.view, corpse) {
            continue;
        }
        let mut evidence = Evidence::with_entity(corpse);
        evidence.places.insert(place);
        emit_candidate(
            candidates,
            GoalKind::LootCorpse { corpse },
            evidence,
            ctx.blocked,
            ctx.current_tick,
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

fn emit_bury_goals(candidates: &mut BTreeMap<GoalKey, GroundedGoal>, ctx: &GenerationContext<'_>) {
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

    for corpse in ctx.view.corpse_entities_at(place) {
        let mut evidence = Evidence::with_entity(corpse);
        evidence.entities.insert(burial_site);
        evidence.places.insert(place);
        emit_candidate(
            candidates,
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            },
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_candidate(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    kind: GoalKind,
    evidence: Evidence,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) {
    if evidence.is_empty() {
        return;
    }

    let key = GoalKey::from(kind);
    if blocked.is_blocked(&key, current_tick) {
        return;
    }

    match candidates.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(GroundedGoal {
                key,
                evidence_entities: evidence.entities,
                evidence_places: evidence.places,
            });
        }
        Entry::Occupied(mut entry) => {
            entry.get_mut().evidence_entities.extend(evidence.entities);
            entry.get_mut().evidence_places.extend(evidence.places);
        }
    }
}

fn acquisition_path_evidence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
    recipes: &RecipeRegistry,
    travel_horizon: u8,
) -> Option<Evidence> {
    let place = place?;
    let mut evidence = Evidence::with_place(place);

    for candidate_place in reachable_places_within_horizon(view, place, travel_horizon) {
        let mut place_evidence = Evidence::with_place(candidate_place);

        for seller in view.agents_selling_at(candidate_place, commodity) {
            if seller != agent {
                place_evidence.entities.insert(seller);
            }
        }
        if let Some(local_lots) =
            local_unpossessed_commodity_evidence(view, candidate_place, commodity)
        {
            place_evidence.merge(local_lots);
        }
        for source in view.resource_sources_at(candidate_place, commodity) {
            place_evidence.entities.insert(source);
        }
        for corpse in view.corpse_entities_at(candidate_place) {
            if corpse_contains_commodity(view, corpse, commodity) {
                place_evidence.entities.insert(corpse);
            }
        }
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
            if let Some(recipe_evidence) =
                recipe_path_evidence(view, agent, Some(candidate_place), recipe)
            {
                place_evidence.merge(recipe_evidence);
            }
        }

        if !place_evidence.is_empty() {
            evidence.merge(place_evidence);
        }
    }

    (!evidence.entities.is_empty()).then_some(evidence)
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

fn recipe_path_evidence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    recipe: &RecipeDefinition,
) -> Option<Evidence> {
    let place = place?;
    let workstation_tag = recipe.required_workstation_tag?;

    for required_tool in &recipe.required_tool_kinds {
        if view.unique_item_count(agent, *required_tool) == 0 {
            return None;
        }
    }

    let workstations = view.matching_workstations_at(place, workstation_tag);
    if workstations.is_empty() {
        return None;
    }

    if recipe.inputs.is_empty() {
        let mut evidence = Evidence::with_place(place);
        for workstation in workstations {
            let &(output_commodity, output_quantity) = recipe.outputs.first()?;
            let source_ok = view.resource_source(workstation).is_some_and(|source| {
                source.commodity == output_commodity && source.available_quantity >= output_quantity
            });
            if source_ok {
                evidence.entities.insert(workstation);
            }
        }
        return (!evidence.entities.is_empty()).then_some(evidence);
    }

    for (commodity, required_quantity) in aggregate_recipe_quantities(&recipe.inputs) {
        if view.commodity_quantity(agent, commodity) < required_quantity {
            return None;
        }
    }

    available_recipe_workstation_evidence(view, agent, Some(place), recipe)
}

fn emit_missing_recipe_input_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
    recipe_id: worldwake_core::RecipeId,
    recipe: &RecipeDefinition,
) {
    if recipe.inputs.is_empty() {
        return;
    }
    if available_recipe_workstation_evidence(ctx.view, ctx.agent, ctx.place, recipe).is_none() {
        return;
    }

    for (commodity, required_quantity) in aggregate_recipe_quantities(&recipe.inputs) {
        if ctx.view.commodity_quantity(ctx.agent, commodity) >= required_quantity {
            continue;
        }
        let Some(evidence) = acquisition_path_evidence(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) else {
            continue;
        };
        emit_candidate(
            candidates,
            GoalKind::AcquireCommodity {
                commodity,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            },
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn available_recipe_workstation_evidence(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    recipe: &RecipeDefinition,
) -> Option<Evidence> {
    let place = place?;
    let workstation_tag = recipe.required_workstation_tag?;

    for required_tool in &recipe.required_tool_kinds {
        if view.unique_item_count(agent, *required_tool) == 0 {
            return None;
        }
    }

    let available_workstations = view
        .matching_workstations_at(place, workstation_tag)
        .into_iter()
        .filter(|workstation| !view.has_production_job(*workstation))
        .collect::<Vec<_>>();
    if available_workstations.is_empty() {
        return None;
    }

    let mut evidence = Evidence::with_place(place);
    evidence.entities.extend(available_workstations);
    Some(evidence)
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
        PoliticalCandidateOmissionReason, PoliticalGoalFamily,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BelievedEntityState, BlockedIntent, BlockedIntentMemory, BlockingFact, BodyPart,
        CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DemandObservation, DemandObservationReason, DriveThresholds, EligibilityRule, EntityId,
        EntityKind, GoalKey, GoalKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
        MerchandiseProfile, MetabolismProfile, OfficeData, PerceptionSource, Permille, Quantity,
        RecipeId, ResourceSource, TellProfile, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, DurationExpr, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView,
    };

    #[derive(Default)]
    struct TestBeliefView {
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
        tell_profiles: BTreeMap<EntityId, TellProfile>,
        office_data: BTreeMap<EntityId, OfficeData>,
        office_holders: BTreeMap<EntityId, EntityId>,
        factions_by_member: BTreeMap<EntityId, Vec<EntityId>>,
        loyalties: BTreeMap<(EntityId, EntityId), Permille>,
        support_declarations: BTreeMap<(EntityId, EntityId), EntityId>,
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity) && !self.dead.contains(&entity)
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

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
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

        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
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

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
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

        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
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

        fn office_data(&self, office: EntityId) -> Option<OfficeData> {
            self.office_data.get(&office).cloned()
        }

        fn office_holder(&self, office: EntityId) -> Option<EntityId> {
            self.office_holders.get(&office).copied()
        }

        fn factions_of(&self, member: EntityId) -> Vec<EntityId> {
            self.factions_by_member
                .get(&member)
                .cloned()
                .unwrap_or_default()
        }

        fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
            self.loyalties.get(&(subject, target)).copied()
        }

        fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
            self.support_declarations.get(&(supporter, office)).copied()
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

    fn believed_state(observed_tick: u64, source: PerceptionSource) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: None,
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

    fn known_entity(subject: EntityId, place: EntityId) -> (EntityId, BelievedEntityState) {
        (
            subject,
            BelievedEntityState {
                last_known_place: Some(place),
                ..believed_state(5, PerceptionSource::DirectObservation)
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
    fn remote_harvest_source_within_travel_horizon_emits_acquire_goal() {
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
            &recipes,
            Tick(5),
            2,
        );
        let goal = candidates
            .candidates
            .iter()
            .find(|candidate| {
                candidate.key.kind
                    == GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Apple,
                        purpose: CommodityPurpose::SelfConsume,
                    }
            })
            .expect("reachable remote harvest source should emit acquire goal");

        assert!(goal.evidence_entities.contains(&workstation));
        assert!(goal.evidence_places.contains(&orchard));
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
    fn blocked_acquire_goal_is_suppressed() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let key = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: key,
                blocking_fact: BlockingFact::NoKnownSeller,
                related_entity: Some(seller),
                related_place: Some(place),
                related_action: None,
                observed_tick: Tick(1),
                expires_tick: Tick(10),
            }],
        };

        let candidates =
            generate_candidates(&view, agent, &blocked, &RecipeRegistry::new(), Tick(5));

        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ));
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
    fn missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal() {
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
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ProduceCommodity { recipe_id }
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
            recipes: &recipes,
            current_tick: Tick(5),
        };
        let mut candidates = BTreeMap::new();

        emit_restock_goals(&mut candidates, &ctx);
        emit_produce_goals(&mut candidates, &ctx, None, None);
        assert!(!contains_goal(
            &candidates.into_values().collect::<Vec<_>>(),
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }
        ));

        let ctx = GenerationContext {
            enterprise: analyze_candidate_enterprise(&view, agent, Some(place)),
            ..ctx
        };
        let mut candidates = BTreeMap::new();

        emit_restock_goals(&mut candidates, &ctx);
        emit_produce_goals(&mut candidates, &ctx, None, None);
        let candidates = candidates.into_values().collect::<Vec<_>>();

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
    fn still_deferred_goal_kinds_are_not_emitted() {
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
        let mut view = TestBeliefView::default();
        view.alive
            .extend([speaker, listener_a, listener_b, crate_lot]);
        view.dead.insert(dead_listener);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener_a, EntityKind::Agent);
        view.entity_kinds.insert(listener_b, EntityKind::Agent);
        view.entity_kinds.insert(dead_listener, EntityKind::Agent);
        view.entity_kinds.insert(crate_lot, EntityKind::ItemLot);
        view.effective_places.insert(speaker, place);
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
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    subject_a,
                    believed_state(8, PerceptionSource::DirectObservation),
                ),
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
                subject: subject_b,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_a,
                subject: subject_a,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_b,
                subject: subject_b,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_b,
                subject: subject_a,
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: dead_listener,
                subject: subject_b,
            }
        ));
        assert!(!contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener: listener_a,
                subject: too_deep,
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
            GoalKind::ShareBelief { listener, subject }
        ));

        view.tell_profiles.insert(speaker, TellProfile::default());
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: GoalKey::from(GoalKind::ShareBelief { listener, subject }),
                blocking_fact: BlockingFact::NoKnownPath,
                related_entity: Some(listener),
                related_place: Some(place),
                related_action: None,
                observed_tick: Tick(10),
                expires_tick: Tick(20),
            }],
        };

        let blocked_candidates =
            generate_candidates(&view, speaker, &blocked, &RecipeRegistry::new(), Tick(11));
        assert!(!contains_goal(
            &blocked_candidates,
            GoalKind::ShareBelief { listener, subject }
        ));
    }

    #[test]
    fn social_candidates_skip_subjects_already_known_to_be_colocated() {
        let speaker = entity(1);
        let listener = entity(2);
        let local_subject = entity(20);
        let remote_subject = entity(21);
        let place = entity(10);
        let remote_place = entity(11);
        let mut view = TestBeliefView::default();
        view.alive
            .extend([speaker, listener, local_subject, remote_subject]);
        view.entity_kinds.insert(speaker, EntityKind::Agent);
        view.entity_kinds.insert(listener, EntityKind::Agent);
        view.entity_kinds.insert(local_subject, EntityKind::Agent);
        view.entity_kinds
            .insert(remote_subject, EntityKind::Facility);
        view.effective_places.insert(speaker, place);
        view.effective_places.insert(local_subject, place);
        view.effective_places.insert(remote_subject, remote_place);
        view.entities_at
            .insert(place, vec![speaker, listener, local_subject]);
        view.tell_profiles.insert(speaker, TellProfile::default());
        view.beliefs.insert(
            speaker,
            vec![
                (
                    local_subject,
                    BelievedEntityState {
                        last_known_place: Some(place),
                        last_known_inventory: BTreeMap::new(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(8),
                        source: PerceptionSource::DirectObservation,
                    },
                ),
                (
                    remote_subject,
                    BelievedEntityState {
                        last_known_place: Some(remote_place),
                        last_known_inventory: BTreeMap::new(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        last_known_courage: None,
                        observed_tick: Tick(9),
                        source: PerceptionSource::DirectObservation,
                    },
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

        assert!(!contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener,
                subject: local_subject,
            }
        ));
        assert!(contains_goal(
            &candidates,
            GoalKind::ShareBelief {
                listener,
                subject: remote_subject,
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
    fn political_candidates_require_known_office_belief_for_generation() {
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
        view.beliefs
            .insert(agent, vec![known_entity(candidate, town)]);

        let without_office_belief = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
        );

        assert!(
            !contains_goal(&without_office_belief, GoalKind::ClaimOffice { office }),
            "unknown offices must not emit ClaimOffice candidates"
        );
        assert!(
            !contains_goal(
                &without_office_belief,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "unknown offices must not emit support candidates even when a loyal candidate is known"
        );

        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let with_office_belief = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
        );

        assert!(contains_goal(
            &with_office_belief,
            GoalKind::ClaimOffice { office }
        ));
        assert!(contains_goal(
            &with_office_belief,
            GoalKind::SupportCandidateForOffice { office, candidate }
        ));
    }

    #[test]
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
        let occupied_with_stale_vacancy = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
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
        let filled = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
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
        let declared = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
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
    fn political_candidates_skip_force_law_offices() {
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
        view.beliefs.insert(
            agent,
            vec![known_entity(office, town), known_entity(candidate, town)],
        );

        let candidates = generate_candidates_with_travel_horizon(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::default(),
            Tick(10),
            6,
        );

        assert!(
            !contains_goal(&candidates.candidates, GoalKind::ClaimOffice { office }),
            "Force-law offices should not emit support-based ClaimOffice goals"
        );
        assert!(
            !contains_goal(
                &candidates.candidates,
                GoalKind::SupportCandidateForOffice { office, candidate }
            ),
            "Force-law offices should not emit support-based support-candidate goals"
        );
        assert!(contains_political_omission(
            &candidates.diagnostics,
            PoliticalGoalFamily::ClaimOffice,
            office,
            None,
            PoliticalCandidateOmissionReason::ForceSuccessionLaw,
        ));
        assert!(contains_political_omission(
            &candidates.diagnostics,
            PoliticalGoalFamily::SupportCandidateForOffice,
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
            &RecipeRegistry::default(),
            Tick(10),
            6,
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
}
