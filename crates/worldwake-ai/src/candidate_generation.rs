use crate::{derive_danger_pressure, enterprise::restock_gap, GroundedGoal};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use worldwake_core::{
    BlockedIntentMemory, CommodityKind, CommodityPurpose, DriveThresholds, EntityId, GoalKey,
    GoalKind, HomeostaticNeeds, Quantity, Tick,
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
    if view.is_dead(agent) || !view.is_alive(agent) {
        return Vec::new();
    }

    let mut candidates = BTreeMap::new();
    let needs = view.homeostatic_needs(agent);
    let thresholds = view.drive_thresholds(agent);
    let ctx = GenerationContext {
        view,
        agent,
        place: view.effective_place(agent),
        blocked,
        recipes,
        current_tick,
    };

    if let (Some(needs), Some(thresholds)) = (needs, thresholds) {
        emit_self_consume_candidates(&mut candidates, &ctx, needs, thresholds);
        emit_sleep_goal(&mut candidates, &ctx, needs, thresholds);
        emit_relieve_goal(&mut candidates, &ctx, needs, thresholds);
        emit_wash_goal(&mut candidates, &ctx, needs, thresholds);
    }

    emit_produce_goals(
        &mut candidates,
        &ctx,
        needs,
        thresholds,
    );
    emit_restock_goals(&mut candidates, &ctx);

    emit_reduce_danger_goal(&mut candidates, &ctx);
    emit_heal_goals(&mut candidates, &ctx);
    emit_loot_goals(&mut candidates, &ctx);

    candidates.into_values().collect()
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

    let already_satisfied = CommodityKind::ALL
        .into_iter()
        .any(|commodity| {
            matches_need(commodity)
                && local_controlled_commodity_exists(ctx.view, ctx.agent, ctx.place, commodity)
        });

    for commodity in CommodityKind::ALL.into_iter().filter(|commodity| matches_need(*commodity)) {
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

        if let Some(evidence) =
            acquisition_path_evidence(ctx.view, ctx.agent, ctx.place, commodity, ctx.recipes)
        {
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
        emit_candidate(candidates, GoalKind::Wash, evidence, ctx.blocked, ctx.current_tick);
    }
}

fn emit_reduce_danger_goal(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    if derive_danger_pressure(ctx.view, ctx.agent).value() == 0 {
        return;
    }

    let mut evidence = Evidence::default();
    if let Some(place) = ctx.place {
        let adjacent = ctx.view.adjacent_places_with_travel_ticks(place);
        if !adjacent.is_empty() {
            evidence.places.insert(place);
            evidence
                .places
                .extend(adjacent.into_iter().map(|(adjacent_place, _)| adjacent_place));
        }
    }
    if ctx.view.commodity_quantity(ctx.agent, CommodityKind::Medicine) > Quantity(0) {
        evidence.entities.extend(local_wounded_targets(
            ctx.view,
            ctx.agent,
            ctx.place,
        ));
    }
    evidence.entities.extend(ctx.view.current_attackers_of(ctx.agent));

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

fn emit_heal_goals(
    candidates: &mut BTreeMap<GoalKey, GroundedGoal>,
    ctx: &GenerationContext<'_>,
) {
    if ctx.view.commodity_quantity(ctx.agent, CommodityKind::Medicine) == Quantity(0) {
        return;
    }

    for target in local_wounded_targets(ctx.view, ctx.agent, ctx.place) {
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
        let Some(mut evidence) = recipe_path_evidence(ctx.view, ctx.agent, ctx.place, recipe) else {
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
                && !local_wounded_targets(ctx.view, ctx.agent, ctx.place).is_empty()
        });
        let serves_restock = recipe.outputs.iter().any(|(commodity, _)| {
            restock_gap(ctx.view, ctx.agent, ctx.place, *commodity).is_some()
        });

        if !(serves_self_consume || serves_treatment || serves_restock) {
            continue;
        }

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
    }
}

fn emit_restock_goals(candidates: &mut BTreeMap<GoalKey, GroundedGoal>, ctx: &GenerationContext<'_>) {
    let Some(profile) = ctx.view.merchandise_profile(ctx.agent) else {
        return;
    };

    for commodity in profile.sale_kinds {
        if restock_gap(ctx.view, ctx.agent, ctx.place, commodity).is_none() {
            continue;
        }
        if let Some(evidence) =
            acquisition_path_evidence(ctx.view, ctx.agent, ctx.place, commodity, ctx.recipes)
        {
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
) -> Option<Evidence> {
    let place = place?;
    let mut evidence = Evidence::with_place(place);

    for seller in view.agents_selling_at(place, commodity) {
        if seller != agent {
            evidence.entities.insert(seller);
        }
    }
    for source in view.resource_sources_at(place, commodity) {
        evidence.entities.insert(source);
    }
    for corpse in view.corpse_entities_at(place) {
        if corpse_contains_commodity(view, corpse, commodity) {
            evidence.entities.insert(corpse);
        }
    }
    for recipe_id in view.known_recipes(agent) {
        let Some(recipe) = recipes.get(recipe_id) else {
            continue;
        };
        if !recipe.outputs.iter().any(|(output, _)| *output == commodity) {
            continue;
        }
        if let Some(recipe_evidence) = recipe_path_evidence(view, agent, Some(place), recipe) {
            evidence.merge(recipe_evidence);
        }
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
            let source_ok = view
                .resource_source(workstation)
                .is_some_and(|source| {
                    source.commodity == output_commodity && source.available_quantity >= output_quantity
                });
            if source_ok {
                evidence.entities.insert(workstation);
            }
        }
        return (!evidence.entities.is_empty()).then_some(evidence);
    }

    let mut evidence = Evidence::with_place(place);
    for (commodity, required_quantity) in aggregate_recipe_quantities(&recipe.inputs) {
        if view.commodity_quantity(agent, commodity) < required_quantity {
            return None;
        }
    }

    let available_workstations = workstations
        .into_iter()
        .filter(|workstation| !view.has_production_job(*workstation))
        .collect::<Vec<_>>();
    if available_workstations.is_empty() {
        return None;
    }

    evidence.entities.extend(available_workstations);
    Some(evidence)
}

fn aggregate_recipe_quantities(entries: &[(CommodityKind, Quantity)]) -> BTreeMap<CommodityKind, Quantity> {
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
    CommodityKind::ALL
        .into_iter()
        .any(|commodity| matches_need(commodity) && local_controlled_commodity_exists(view, agent, place, commodity))
}

fn corpse_contains_commodity(view: &dyn BeliefView, corpse: EntityId, commodity: CommodityKind) -> bool {
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
    use super::generate_candidates;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BlockedIntent, BlockedIntentMemory, BlockingFact, BodyPart, CommodityConsumableProfile,
        CommodityKind, CommodityPurpose, DemandObservation, DemandObservationReason,
        DriveThresholds, EntityId, EntityKind, GoalKey, GoalKind, HomeostaticNeeds,
        InTransitOnEdge, MerchandiseProfile, Permille, Quantity, RecipeId, ResourceSource, Tick,
        TickRange, UniqueItemKind, WorkstationTag, Wound, WoundCause, WoundId,
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
            self.adjacent_places.get(&place).cloned().unwrap_or_default()
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

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile> {
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

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn is_incapacitated(&self, entity: EntityId) -> bool {
            self.incapacitated.contains(&entity)
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds.get(&entity).is_some_and(|wounds| !wounds.is_empty())
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.homeostatic_needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.drive_thresholds.get(&agent).copied()
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

        fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
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
        EntityId { slot, generation: 1 }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn hunger(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(value), pm(0), pm(0), pm(0), pm(0))
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
        candidates.iter().any(|candidate| candidate.key.kind == goal)
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
        view.drive_thresholds.insert(agent, DriveThresholds::default());
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
        view.drive_thresholds.insert(agent, DriveThresholds::default());
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
    fn hunger_below_low_band_emits_no_hunger_goals() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.homeostatic_needs.insert(agent, hunger(50));
        view.drive_thresholds.insert(agent, DriveThresholds::default());

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
        view.drive_thresholds.insert(agent, DriveThresholds::default());
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: key,
                blocking_fact: BlockingFact::NoKnownSeller,
                related_entity: Some(seller),
                related_place: Some(place),
                observed_tick: Tick(1),
                expires_tick: Tick(10),
            }],
        };

        let candidates = generate_candidates(&view, agent, &blocked, &RecipeRegistry::new(), Tick(5));

        assert!(!contains_goal(
            &candidates,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
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
        view.drive_thresholds.insert(agent, DriveThresholds::default());

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
        view.drive_thresholds.insert(agent, DriveThresholds::default());
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
        view.drive_thresholds.insert(agent, DriveThresholds::default());
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
        assert!(!contains_goal(
            &none,
            GoalKind::Heal { target: patient }
        ));

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
    fn satisfiable_recipe_with_current_need_emits_produce_goal() {
        let agent = entity(1);
        let place = entity(10);
        let workstation = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.effective_places.insert(agent, place);
        view.homeostatic_needs.insert(agent, hunger(250));
        view.drive_thresholds.insert(agent, DriveThresholds::default());
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

        assert!(contains_goal(
            &candidates,
            GoalKind::LootCorpse { corpse }
        ));
    }

    #[test]
    fn deferred_goal_kinds_are_not_emitted() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.homeostatic_needs.insert(agent, fatigue(250));
        view.drive_thresholds.insert(agent, DriveThresholds::default());

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
                    | GoalKind::MoveCargo { .. }
                    | GoalKind::BuryCorpse { .. }
            )
        }));
    }
}
