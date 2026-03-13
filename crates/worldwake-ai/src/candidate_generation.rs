use crate::{
    derive_danger_pressure,
    enterprise::{analyze_candidate_enterprise, restock_gap_at_destination, EnterpriseSignals},
    GroundedGoal,
};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque};
use worldwake_core::{
    load_per_unit, BlockedIntentMemory, CommodityKind, CommodityPurpose, DriveThresholds, EntityId,
    GoalKey, GoalKind, HomeostaticNeeds, Quantity, Tick,
};
use worldwake_sim::{BeliefView, RecipeDefinition, RecipeRegistry};

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
    view: &'a dyn BeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    travel_horizon: u8,
    enterprise: EnterpriseSignals,
    blocked: &'a BlockedIntentMemory,
    recipes: &'a RecipeRegistry,
    current_tick: Tick,
}

#[must_use]
pub fn generate_candidates(
    view: &dyn BeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal> {
    generate_candidates_with_travel_horizon(view, agent, blocked, recipes, current_tick, 6)
}

#[must_use]
pub fn generate_candidates_with_travel_horizon(
    view: &dyn BeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,
) -> Vec<GroundedGoal> {
    if view.is_dead(agent) || !view.is_alive(agent) {
        return Vec::new();
    }

    let mut candidates = BTreeMap::new();
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

    candidates.into_values().collect()
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
    emit_treatment_candidates(candidates, ctx);
    emit_heal_goals(candidates, ctx);
    emit_loot_goals(candidates, ctx);
    emit_bury_goals(candidates, ctx);
}

fn emit_engage_hostile_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    if ctx.view.drive_thresholds(ctx.agent).is_some_and(|thresholds| {
        derive_danger_pressure(ctx.view, ctx.agent) >= thresholds.danger.high()
    }) {
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

    let already_satisfied = CommodityKind::ALL.into_iter().any(|commodity| {
        matches_need(commodity)
            && local_controlled_commodity_exists(ctx.view, ctx.agent, ctx.place, commodity)
    });

    for commodity in CommodityKind::ALL
        .into_iter()
        .filter(|commodity| matches_need(*commodity))
    {
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

fn emit_heal_goals(candidates: &mut BTreeMap<GoalKey, GroundedGoal>, ctx: &GenerationContext<'_>) {
    if ctx
        .view
        .commodity_quantity(ctx.agent, CommodityKind::Medicine)
        == Quantity(0)
    {
        return;
    }

    for target in local_heal_targets(ctx.view, ctx.agent, ctx.place) {
        let mut evidence = Evidence::with_entity(target);
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        emit_candidate(
            candidates,
            GoalKind::Heal { target },
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn emit_treatment_candidates(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    let heal_targets = local_heal_targets(ctx.view, ctx.agent, ctx.place);
    if heal_targets.is_empty() {
        return;
    }

    for commodity in CommodityKind::ALL
        .into_iter()
        .filter(|commodity| commodity.spec().treatment_profile.is_some())
    {
        if ctx.view.commodity_quantity(ctx.agent, commodity) > Quantity(0)
            || local_controlled_commodity_exists(ctx.view, ctx.agent, ctx.place, commodity)
        {
            continue;
        }

        let Some(mut evidence) = acquisition_path_evidence(
            ctx.view,
            ctx.agent,
            ctx.place,
            commodity,
            ctx.recipes,
            ctx.travel_horizon,
        ) else {
            continue;
        };

        evidence.entities.extend(heal_targets.iter().copied());
        if let Some(place) = ctx.place {
            evidence.places.insert(place);
        }
        emit_candidate(
            candidates,
            GoalKind::AcquireCommodity {
                commodity,
                purpose: CommodityPurpose::Treatment,
            },
            evidence,
            ctx.blocked,
            ctx.current_tick,
        );
    }
}

fn local_hostility_targets(
    view: &dyn BeliefView,
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
        let serves_treatment = recipe.outputs.iter().any(|(commodity, _)| {
            commodity.spec().treatment_profile.is_some()
                && ctx.view.commodity_quantity(ctx.agent, *commodity) == Quantity(0)
                && !local_heal_targets(ctx.view, ctx.agent, ctx.place).is_empty()
        });
        let serves_restock = recipe
            .outputs
            .iter()
            .any(|(commodity, _)| ctx.enterprise.restock_gap(*commodity).is_some());

        if !(serves_self_consume || serves_treatment || serves_restock) {
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
    view: &dyn BeliefView,
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
        if ctx.view.direct_possessions(corpse).is_empty() {
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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

fn local_heal_targets(
    view: &dyn BeliefView,
    agent: EntityId,
    place: Option<EntityId>,
) -> Vec<EntityId> {
    local_wounded_targets(view, agent, place)
        .into_iter()
        .filter(|target| *target != agent)
        .collect()
}

fn local_controlled_commodity_exists(
    view: &dyn BeliefView,
    agent: EntityId,
    place: Option<EntityId>,
    commodity: CommodityKind,
) -> bool {
    local_controlled_commodity_evidence(view, agent, place, commodity).is_some()
}

fn local_controlled_commodity_evidence(
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
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
    view: &dyn BeliefView,
    corpse: EntityId,
    commodity: CommodityKind,
) -> bool {
    view.direct_possessions(corpse)
        .into_iter()
        .any(|entity| view.item_lot_commodity(entity) == Some(commodity))
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
        GenerationContext,
    };
    use crate::enterprise::{analyze_candidate_enterprise, EnterpriseSignals};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BlockedIntent, BlockedIntentMemory, BlockingFact, BodyPart, CombatProfile,
        CommodityConsumableProfile, CommodityKind, CommodityPurpose, DemandObservation,
        DemandObservationReason, DriveThresholds, EntityId, EntityKind, GoalKey, GoalKind,
        HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile,
        Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, BeliefView, DurationExpr, RecipeDefinition, RecipeRegistry,
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
    }

    impl BeliefView for TestBeliefView {
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
    fn heal_requires_medicine_and_local_wounded_target() {
        let agent = entity(1);
        let patient = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient]);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(patient, place);
        view.entities_at.insert(place, vec![agent, patient]);
        view.wounds.insert(patient, vec![wound(100)]);

        let none = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );
        assert!(!contains_goal(&none, GoalKind::Heal { target: patient }));

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
            GoalKind::Heal { target: patient }
        ));
    }

    #[test]
    fn self_wounds_do_not_emit_treatment_acquire_goal_without_a_healable_other() {
        let agent = entity(1);
        let seller = entity(2);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(seller, place);
        view.entities_at.insert(place, vec![agent, seller]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.wounds.insert(agent, vec![wound(100)]);
        view.sellers
            .insert((place, CommodityKind::Medicine), vec![seller]);

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
                commodity: CommodityKind::Medicine,
                purpose: CommodityPurpose::Treatment,
            }
        ));
    }

    #[test]
    fn local_wounded_other_can_emit_treatment_acquire_goal() {
        let agent = entity(1);
        let patient = entity(2);
        let seller = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, patient, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(patient, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(patient, place);
        view.effective_places.insert(seller, place);
        view.entities_at.insert(place, vec![agent, patient, seller]);
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.wounds.insert(patient, vec![wound(100)]);
        view.sellers
            .insert((place, CommodityKind::Medicine), vec![seller]);

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
                commodity: CommodityKind::Medicine,
                purpose: CommodityPurpose::Treatment,
            }
        ));
    }

    #[test]
    fn treatment_acquire_goal_is_suppressed_when_medicine_is_already_controlled() {
        let agent = entity(1);
        let place = entity(10);
        let medicine = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, medicine]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(medicine, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(medicine, place);
        view.entities_at.insert(place, vec![agent, medicine]);
        view.direct_possessions.insert(agent, vec![medicine]);
        view.direct_possessors.insert(medicine, agent);
        view.lot_commodities.insert(medicine, CommodityKind::Medicine);
        view.commodity_quantities
            .insert((agent, CommodityKind::Medicine), Quantity(1));
        view.commodity_quantities
            .insert((medicine, CommodityKind::Medicine), Quantity(1));
        view.controllable.insert((agent, medicine));
        view.drive_thresholds
            .insert(agent, DriveThresholds::default());
        view.wounds.insert(agent, vec![wound(100)]);

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
                commodity: CommodityKind::Medicine,
                purpose: CommodityPurpose::Treatment,
            }
        ));
    }

    #[test]
    fn self_wounds_do_not_emit_heal_goal() {
        let agent = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.entities_at.insert(place, vec![agent]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Medicine), Quantity(1));
        view.wounds.insert(agent, vec![wound(100)]);

        let candidates = generate_candidates(
            &view,
            agent,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            Tick(5),
        );

        assert!(!contains_goal(&candidates, GoalKind::Heal { target: agent }));
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

        assert!(!candidates.iter().any(|candidate| {
            matches!(
                candidate.key.kind,
                GoalKind::SellCommodity { .. }
            )
        }));
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
}
