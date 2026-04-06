use crate::{GoalBeliefView, RecipeDefinition};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use worldwake_core::{
    CommodityKind, CommodityValuationProfile, EntityId, Permille, RecipeId, WorkstationTag,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CommodityOpportunityBreakdown {
    pub direct_survival_score: u32,
    pub treatment_score: u32,
    pub enterprise_score: u32,
    pub indirect_recipe_score: u32,
}

#[must_use]
pub fn commodity_opportunity_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> CommodityOpportunityBreakdown {
    let direct_survival_score =
        direct_survival_score(actor, commodity, belief, holdings, local_alternatives);
    let treatment_score = treatment_score(actor, commodity, belief, holdings, local_alternatives);
    let enterprise_score = enterprise_score(actor, commodity, belief, holdings, local_alternatives);

    CommodityOpportunityBreakdown {
        direct_survival_score,
        treatment_score,
        enterprise_score,
        indirect_recipe_score: indirect_recipe_score(
            actor,
            commodity,
            belief,
            holdings,
            local_alternatives,
            &mut BTreeSet::new(),
        ),
    }
}

fn direct_survival_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    let Some(profile) = commodity.spec().consumable_profile else {
        return 0;
    };
    let Some(needs) = belief.homeostatic_needs(actor) else {
        return 0;
    };

    let quantity = accessible_quantity(holdings, local_alternatives, commodity);
    let hunger_relief = quantity * u64::from(profile.hunger_relief_per_unit.value());
    let thirst_relief = quantity * u64::from(profile.thirst_relief_per_unit.value());

    saturating_u64_to_u32(
        hunger_relief.min(u64::from(needs.hunger.value()))
            + thirst_relief.min(u64::from(needs.thirst.value())),
    )
}

fn treatment_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    if commodity.spec().treatment_profile.is_none() {
        return 0;
    }

    let wounds = belief.wounds(actor);
    if wounds.is_empty() {
        return 0;
    }

    let total_severity = wounds
        .iter()
        .map(|wound| u64::from(wound.severity.value()))
        .sum::<u64>();
    let wound_count = wounds.len() as u64;
    let accessible_medicine = accessible_quantity(holdings, local_alternatives, commodity);

    saturating_u64_to_u32(accessible_medicine.min(wound_count) * total_severity)
}

fn enterprise_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> u32 {
    let remembered_quantity = belief
        .demand_memory(actor)
        .into_iter()
        .filter(|observation| observation.commodity == commodity)
        .map(|observation| u64::from(observation.quantity.0))
        .sum::<u64>();

    if remembered_quantity == 0 {
        return 0;
    }

    saturating_u64_to_u32(
        accessible_quantity(holdings, local_alternatives, commodity).min(remembered_quantity),
    )
}

fn indirect_recipe_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    path: &mut BTreeSet<(CommodityKind, u8)>,
) -> u32 {
    let Some(profile) = belief.commodity_valuation_profile(actor) else {
        return 0;
    };

    indirect_recipe_score_with_profile(
        actor,
        commodity,
        belief,
        holdings,
        local_alternatives,
        profile,
        profile.recipe_opportunity_depth.get(),
        path,
    )
}

#[allow(clippy::too_many_arguments)]
fn indirect_recipe_score_with_profile(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    profile: CommodityValuationProfile,
    remaining_depth: u8,
    path: &mut BTreeSet<(CommodityKind, u8)>,
) -> u32 {
    if remaining_depth == 0 || !path.insert((commodity, remaining_depth)) {
        return 0;
    }

    let result = best_recipe_opportunity(
        actor,
        commodity,
        belief,
        holdings,
        local_alternatives,
        profile,
        remaining_depth,
        path,
    )
    .map_or(0, |best| best.value);

    path.remove(&(commodity, remaining_depth));
    result
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct BestOpportunity {
    value: u32,
    steps: u8,
    recipe_id: RecipeId,
}

#[allow(clippy::too_many_arguments)]
fn best_recipe_opportunity(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    profile: CommodityValuationProfile,
    remaining_depth: u8,
    path: &mut BTreeSet<(CommodityKind, u8)>,
) -> Option<BestOpportunity> {
    if accessible_quantity(holdings, local_alternatives, commodity) == 0 {
        return None;
    }

    let mut best = None;

    for recipe_id in belief.known_recipes(actor) {
        let Some(recipe) = belief.recipe_definition(recipe_id) else {
            continue;
        };

        if !recipe
            .inputs
            .iter()
            .any(|(input_commodity, _)| *input_commodity == commodity)
            || !workstation_reachable(actor, recipe.required_workstation_tag, belief, profile)
            || !sibling_inputs_satisfiable(
                actor,
                commodity,
                &recipe,
                belief,
                holdings,
                local_alternatives,
                profile,
                remaining_depth,
                path,
            )
        {
            continue;
        }

        let Some(output_value) = recipe_output_value(
            actor,
            &recipe,
            belief,
            holdings,
            local_alternatives,
            profile,
            remaining_depth,
            path,
        ) else {
            continue;
        };

        let candidate = BestOpportunity {
            value: apply_decay(output_value, profile.indirect_value_decay_per_step),
            steps: profile
                .recipe_opportunity_depth
                .get()
                .saturating_sub(remaining_depth)
                .saturating_add(1),
            recipe_id,
        };

        best = match best {
            None => Some(candidate),
            Some(current) if better_opportunity(candidate, current) => Some(candidate),
            Some(current) => Some(current),
        };
    }

    best
}

fn workstation_reachable(
    actor: EntityId,
    workstation_tag: Option<WorkstationTag>,
    belief: &dyn GoalBeliefView,
    profile: CommodityValuationProfile,
) -> bool {
    let Some(tag) = workstation_tag else {
        return true;
    };
    let Some(origin) = belief.effective_place(actor) else {
        return false;
    };

    if !belief.matching_workstations_at(origin, tag).is_empty() {
        return true;
    }
    if profile.recipe_place_horizon == 0 {
        return false;
    }

    let mut seen = BTreeSet::from([origin]);
    let mut frontier = VecDeque::from([(origin, 0_u8)]);
    while let Some((place, hops)) = frontier.pop_front() {
        if hops >= profile.recipe_place_horizon {
            continue;
        }
        for (adjacent, _) in belief.adjacent_places_with_travel_ticks(place) {
            if !seen.insert(adjacent) {
                continue;
            }
            if !belief.matching_workstations_at(adjacent, tag).is_empty() {
                return true;
            }
            frontier.push_back((adjacent, hops.saturating_add(1)));
        }
    }

    false
}

#[allow(clippy::too_many_arguments)]
fn sibling_inputs_satisfiable(
    actor: EntityId,
    target_commodity: CommodityKind,
    recipe: &RecipeDefinition,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    profile: CommodityValuationProfile,
    remaining_depth: u8,
    path: &mut BTreeSet<(CommodityKind, u8)>,
) -> bool {
    recipe.inputs.iter().all(|(input_commodity, quantity)| {
        if *input_commodity == target_commodity {
            return true;
        }

        if accessible_quantity(holdings, local_alternatives, *input_commodity)
            >= u64::from(quantity.0)
        {
            return true;
        }

        if remaining_depth <= 1 {
            return false;
        }

        indirect_recipe_score_with_profile(
            actor,
            *input_commodity,
            belief,
            holdings,
            local_alternatives,
            profile,
            remaining_depth.saturating_sub(1),
            path,
        ) > 0
    })
}

#[allow(clippy::too_many_arguments)]
fn recipe_output_value(
    actor: EntityId,
    recipe: &RecipeDefinition,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    profile: CommodityValuationProfile,
    remaining_depth: u8,
    path: &mut BTreeSet<(CommodityKind, u8)>,
) -> Option<u32> {
    let mut produced_holdings = holdings.clone();
    for (output_commodity, quantity) in &recipe.outputs {
        let entry = produced_holdings.entry(*output_commodity).or_insert(0);
        *entry = entry.saturating_add(quantity.0);
    }

    recipe
        .outputs
        .iter()
        .map(|(output_commodity, _)| {
            let direct_total = direct_survival_score(
                actor,
                *output_commodity,
                belief,
                &produced_holdings,
                local_alternatives,
            )
            .saturating_add(treatment_score(
                actor,
                *output_commodity,
                belief,
                &produced_holdings,
                local_alternatives,
            ))
            .saturating_add(enterprise_score(
                actor,
                *output_commodity,
                belief,
                &produced_holdings,
                local_alternatives,
            ));

            let recursive_total = if remaining_depth > 1 {
                indirect_recipe_score_with_profile(
                    actor,
                    *output_commodity,
                    belief,
                    &produced_holdings,
                    local_alternatives,
                    profile,
                    remaining_depth.saturating_sub(1),
                    path,
                )
            } else {
                0
            };

            direct_total.max(recursive_total)
        })
        .max()
        .filter(|value| *value > 0)
}

fn better_opportunity(candidate: BestOpportunity, current: BestOpportunity) -> bool {
    candidate.value > current.value
        || (candidate.value == current.value
            && (candidate.steps < current.steps
                || (candidate.steps == current.steps && candidate.recipe_id < current.recipe_id)))
}

fn accessible_quantity(
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
    commodity: CommodityKind,
) -> u64 {
    let held = u64::from(holdings.get(&commodity).copied().unwrap_or(0));
    let alternatives = if commodity == CommodityKind::Coin {
        0
    } else {
        u64::from(local_alternatives.get(&commodity).copied().unwrap_or(0))
    };
    held + alternatives
}

fn apply_decay(value: u32, decay: Permille) -> u32 {
    let retained = 1000_u64.saturating_sub(u64::from(decay.value()));
    saturating_u64_to_u32((u64::from(value) * retained) / 1000)
}

fn saturating_u64_to_u32(value: u64) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{CommodityOpportunityBreakdown, commodity_opportunity_score};
    use crate::{GoalBeliefView, RecipeDefinition, RecipeRegistry};
    use std::collections::BTreeMap;
    use std::num::{NonZeroU8, NonZeroU32};
    use worldwake_core::{
        BeliefConfidencePolicy, BodyCostPerTick, BodyPart, CommodityKind,
        CommodityValuationProfile, DemandObservation, DemandObservationReason, DriveThresholds,
        EntityId, EntityKind, HomeostaticNeeds, LoadUnits, MerchandiseProfile, Permille, Quantity,
        RecipeId, ResourceSource, Tick, WorkstationTag, Wound, WoundCause,
    };

    #[derive(Default)]
    struct StubBeliefView {
        needs: Option<HomeostaticNeeds>,
        wounds: Vec<Wound>,
        demand_memory: Vec<DemandObservation>,
        commodity_valuation_profile: Option<CommodityValuationProfile>,
        known_recipes: Vec<RecipeId>,
        recipe_definitions: BTreeMap<RecipeId, RecipeDefinition>,
        effective_places: BTreeMap<EntityId, EntityId>,
        adjacent_places: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        workstations_by_place: BTreeMap<(EntityId, WorkstationTag), Vec<EntityId>>,
    }

    impl GoalBeliefView for StubBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }

        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            Some(EntityKind::Agent)
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, recipe: RecipeId) -> bool {
            self.known_recipes.contains(&recipe)
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            self.known_recipes.clone()
        }

        fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
            self.recipe_definitions.get(&recipe).cloned()
        }

        fn unique_item_count(
            &self,
            _holder: EntityId,
            _kind: worldwake_core::UniqueItemKind,
        ) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
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

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<worldwake_core::CommodityConsumableProfile> {
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

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
            self.workstations_by_place
                .get(&(place, tag))
                .cloned()
                .unwrap_or_default()
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

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            !self.wounds.is_empty()
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn commodity_valuation_profile(
            &self,
            _agent: EntityId,
        ) -> Option<CommodityValuationProfile> {
            self.commodity_valuation_profile
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            self.wounds.clone()
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

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.clone()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn actor() -> EntityId {
        EntityId {
            slot: 1,
            generation: 0,
        }
    }

    fn holdings(entries: &[(CommodityKind, u32)]) -> BTreeMap<CommodityKind, u32> {
        entries.iter().copied().collect()
    }

    fn demand_observation(commodity: CommodityKind, quantity: u32) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(quantity),
            place: EntityId {
                slot: 9,
                generation: 0,
            },
            tick: Tick(12),
            counterparty: None,
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn wound(severity: u16) -> Wound {
        Wound {
            id: worldwake_core::WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
            severity: Permille::new(severity).unwrap(),
            inflicted_at: Tick(5),
            bleed_rate_per_tick: Permille::new(0).unwrap(),
        }
    }

    fn valuation_profile(depth: u8, horizon: u8, decay: u16) -> CommodityValuationProfile {
        CommodityValuationProfile {
            recipe_opportunity_depth: NonZeroU8::new(depth).unwrap(),
            recipe_place_horizon: horizon,
            indirect_value_decay_per_step: Permille::new(decay).unwrap(),
        }
    }

    fn recipe(
        name: &str,
        inputs: Vec<(CommodityKind, u32)>,
        outputs: Vec<(CommodityKind, u32)>,
        workstation: Option<WorkstationTag>,
    ) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs: inputs
                .into_iter()
                .map(|(commodity, qty)| (commodity, Quantity(qty)))
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(commodity, qty)| (commodity, Quantity(qty)))
                .collect(),
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: workstation,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        }
    }

    fn with_actor_place(mut view: StubBeliefView, place: EntityId) -> StubBeliefView {
        view.effective_places.insert(actor(), place);
        view
    }

    #[test]
    fn commodity_opportunity_survival_score_tracks_need_pressure_for_consumables() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(300).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &holdings(&[(CommodityKind::Bread, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.direct_survival_score, 260);
        assert_eq!(breakdown.treatment_score, 0);
        assert_eq!(breakdown.enterprise_score, 0);
    }

    #[test]
    fn commodity_opportunity_non_consumable_has_no_survival_score() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(800).unwrap(),
                Permille::new(600).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Firewood, 2)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.direct_survival_score, 0);
    }

    #[test]
    fn commodity_opportunity_treatment_score_tracks_wound_severity_for_medicine() {
        let view = StubBeliefView {
            wounds: vec![wound(300), wound(200)],
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Medicine,
            &view,
            &holdings(&[(CommodityKind::Medicine, 1)]),
            &holdings(&[(CommodityKind::Medicine, 1)]),
        );

        assert_eq!(breakdown.treatment_score, 1000);
        assert_eq!(breakdown.direct_survival_score, 0);
    }

    #[test]
    fn commodity_opportunity_treatment_score_zero_without_wounds() {
        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Medicine,
            &StubBeliefView::default(),
            &holdings(&[(CommodityKind::Medicine, 3)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.treatment_score, 0);
    }

    #[test]
    fn commodity_opportunity_enterprise_score_tracks_remembered_demand() {
        let view = StubBeliefView {
            demand_memory: vec![
                demand_observation(CommodityKind::Bread, 3),
                demand_observation(CommodityKind::Apple, 4),
            ],
            ..Default::default()
        };

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Bread,
            &view,
            &holdings(&[(CommodityKind::Bread, 1)]),
            &holdings(&[(CommodityKind::Bread, 1)]),
        );

        assert_eq!(breakdown.enterprise_score, 2);
    }

    #[test]
    fn firewood_gains_indirect_value_for_known_reachable_bread_recipe() {
        let origin = EntityId {
            slot: 10,
            generation: 0,
        };
        let mill = EntityId {
            slot: 11,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Grain, 1), (CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let mut view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 1, 100)),
                known_recipes: vec![bake_bread],
                recipe_definitions: BTreeMap::from([(
                    bake_bread,
                    recipes.get(bake_bread).unwrap().clone(),
                )]),
                ..Default::default()
            },
            origin,
        );
        view.adjacent_places
            .insert(origin, vec![(mill, NonZeroU32::new(1).unwrap())]);
        view.workstations_by_place.insert(
            (mill, WorkstationTag::Mill),
            vec![EntityId {
                slot: 12,
                generation: 0,
            }],
        );

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Grain, 1), (CommodityKind::Firewood, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.direct_survival_score, 0);
        assert_eq!(breakdown.indirect_recipe_score, 234);
    }

    #[test]
    fn no_indirect_value_when_required_workstation_not_reachable() {
        let origin = EntityId {
            slot: 20,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Grain, 1), (CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
                known_recipes: vec![bake_bread],
                recipe_definitions: BTreeMap::from([(
                    bake_bread,
                    recipes.get(bake_bread).unwrap().clone(),
                )]),
                ..Default::default()
            },
            origin,
        );

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Grain, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.indirect_recipe_score, 0);
    }

    #[test]
    fn no_indirect_value_when_recipe_is_unknown() {
        let origin = EntityId {
            slot: 30,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Grain, 1), (CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let mut view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 1, 100)),
                recipe_definitions: BTreeMap::from([(
                    bake_bread,
                    recipes.get(bake_bread).unwrap().clone(),
                )]),
                ..Default::default()
            },
            origin,
        );
        view.workstations_by_place.insert(
            (origin, WorkstationTag::Mill),
            vec![EntityId {
                slot: 31,
                generation: 0,
            }],
        );

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Grain, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.indirect_recipe_score, 0);
    }

    #[test]
    fn sibling_input_unavailability_blocks_indirect_value() {
        let origin = EntityId {
            slot: 40,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Grain, 1), (CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let mut view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
                known_recipes: vec![bake_bread],
                recipe_definitions: BTreeMap::from([(
                    bake_bread,
                    recipes.get(bake_bread).unwrap().clone(),
                )]),
                ..Default::default()
            },
            origin,
        );
        view.workstations_by_place.insert(
            (origin, WorkstationTag::Mill),
            vec![EntityId {
                slot: 41,
                generation: 0,
            }],
        );

        let breakdown = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &BTreeMap::new(),
            &BTreeMap::new(),
        );

        assert_eq!(breakdown.indirect_recipe_score, 0);
    }

    #[test]
    fn multistep_chain_propagates_with_decay_and_respects_depth_limit() {
        let origin = EntityId {
            slot: 50,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Waste, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let make_waste = recipes.register(recipe(
            "Make Waste",
            vec![(CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Waste, 1)],
            Some(WorkstationTag::Forge),
        ));
        let mut deep_view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
                known_recipes: vec![bake_bread, make_waste],
                recipe_definitions: BTreeMap::from([
                    (bake_bread, recipes.get(bake_bread).unwrap().clone()),
                    (make_waste, recipes.get(make_waste).unwrap().clone()),
                ]),
                ..Default::default()
            },
            origin,
        );
        deep_view.workstations_by_place.insert(
            (origin, WorkstationTag::Mill),
            vec![EntityId {
                slot: 51,
                generation: 0,
            }],
        );
        deep_view.workstations_by_place.insert(
            (origin, WorkstationTag::Forge),
            vec![EntityId {
                slot: 52,
                generation: 0,
            }],
        );

        let waste = commodity_opportunity_score(
            actor(),
            CommodityKind::Waste,
            &deep_view,
            &holdings(&[(CommodityKind::Waste, 1)]),
            &BTreeMap::new(),
        );
        let firewood = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &deep_view,
            &holdings(&[(CommodityKind::Firewood, 1)]),
            &BTreeMap::new(),
        );

        let shallow_view = StubBeliefView {
            commodity_valuation_profile: Some(valuation_profile(1, 0, 100)),
            ..deep_view
        };
        let shallow = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &shallow_view,
            &holdings(&[(CommodityKind::Firewood, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(waste.indirect_recipe_score, 234);
        assert_eq!(firewood.indirect_recipe_score, 210);
        assert!(firewood.indirect_recipe_score < waste.indirect_recipe_score);
        assert_eq!(shallow.indirect_recipe_score, 0);
    }

    #[test]
    fn deterministic_best_path_prefers_higher_value_and_does_not_sum_paths() {
        let origin = EntityId {
            slot: 60,
            generation: 0,
        };
        let mut recipes = RecipeRegistry::new();
        let bake_bread = recipes.register(recipe(
            "Bake Bread",
            vec![(CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Bread, 1)],
            Some(WorkstationTag::Mill),
        ));
        let boil_water = recipes.register(recipe(
            "Boil Water",
            vec![(CommodityKind::Firewood, 1)],
            vec![(CommodityKind::Water, 1)],
            Some(WorkstationTag::Forge),
        ));
        let mut view = with_actor_place(
            StubBeliefView {
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(300).unwrap(),
                    Permille::new(200).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                    Permille::new(0).unwrap(),
                )),
                commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
                known_recipes: vec![bake_bread, boil_water],
                recipe_definitions: BTreeMap::from([
                    (bake_bread, recipes.get(bake_bread).unwrap().clone()),
                    (boil_water, recipes.get(boil_water).unwrap().clone()),
                ]),
                ..Default::default()
            },
            origin,
        );
        view.workstations_by_place.insert(
            (origin, WorkstationTag::Mill),
            vec![EntityId {
                slot: 61,
                generation: 0,
            }],
        );
        view.workstations_by_place.insert(
            (origin, WorkstationTag::Forge),
            vec![EntityId {
                slot: 62,
                generation: 0,
            }],
        );

        let first = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Firewood, 1)]),
            &BTreeMap::new(),
        );
        let second = commodity_opportunity_score(
            actor(),
            CommodityKind::Firewood,
            &view,
            &holdings(&[(CommodityKind::Firewood, 1)]),
            &BTreeMap::new(),
        );

        assert_eq!(first, second);
        assert_eq!(first.indirect_recipe_score, 234);
    }

    #[test]
    fn commodity_opportunity_is_deterministic_for_identical_inputs() {
        let view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new(
                Permille::new(400).unwrap(),
                Permille::new(150).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            )),
            wounds: vec![wound(250)],
            demand_memory: vec![demand_observation(CommodityKind::Bread, 2)],
            ..Default::default()
        };
        let held = holdings(&[(CommodityKind::Bread, 1), (CommodityKind::Medicine, 1)]);
        let alternatives = holdings(&[(CommodityKind::Bread, 1)]);
        let first =
            commodity_opportunity_score(actor(), CommodityKind::Bread, &view, &held, &alternatives);
        let second =
            commodity_opportunity_score(actor(), CommodityKind::Bread, &view, &held, &alternatives);

        assert_eq!(first, second);
        assert_eq!(
            first,
            CommodityOpportunityBreakdown {
                direct_survival_score: 400,
                treatment_score: 0,
                enterprise_score: 2,
                indirect_recipe_score: 0,
            }
        );
    }
}
